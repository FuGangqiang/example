extern crate mio;
extern crate slab;

use std::net::SocketAddr;
use std::io::{Read, Write, ErrorKind};
use mio::*;
use mio::tcp::TcpListener;

type Slab<T> = slab::Slab<T, Token>;

const SERVER_TOKEN: Token = Token(::std::usize::MAX-1);


fn main() {
    let mut args = ::std::env::args();
    let cmd = args.next().unwrap();
    let port = args.next().expect(&format!("Usage: {} [port]", cmd));
    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().expect("argument format error: port");
    let server = TcpListener::bind(&addr).expect("socket binding error");
    let poll = Poll::new().expect("poll create error");
    let mut conns = Slab::with_capacity(1024);
    let mut events = Events::with_capacity(1024);
    let mut buf: [u8; 1024] = [0; 1024];
    let stdout = ::std::io::stdout();

    poll.register(&server, SERVER_TOKEN, Ready::readable(), PollOpt::edge())
        .expect("poll register error");
    // the event loop
    loop {
        poll.poll(&mut events, None).expect("poll error");
        for event in events.iter() {
            let (token, kind) = (event.token(), event.kind());

            if kind.is_error() || kind.is_hup() || !kind.is_readable() {
                println!("kind error");
                if token == SERVER_TOKEN {
                    ::std::process::exit(1);
                }
                conns.remove(token);
            } else if token == SERVER_TOKEN {
                loop {
                    let sock = match server.accept() {
                        Ok((sock, addr)) => {
                            println!("Accepted connection: {}", addr);
                            sock
                        }
                        Err(_) => break
                    };
                    let new_token = conns.insert(sock).expect("add connection error");
                    poll.register(&conns[new_token], new_token, Ready::readable(), PollOpt::edge())
                        .expect("poll register error");

                }
            } else {
                let mut need_to_close = false;
                {
                    let ref mut client = conns[token];
                    loop {
                        match client.read(&mut buf) {
                            Ok(n) => {
                                if n == 0 {
                                    need_to_close = true;
                                    break;
                                } else {
                                    let mut handle = stdout.lock();
                                    handle.write(&buf[..n]).expect("write error");
                                    handle.flush().expect("flush error");
                                }
                            },
                            Err(e) => {
                                if e.kind() != ErrorKind::WouldBlock {
                                    need_to_close = true;
                                }
                                break;
                            }
                        }
                    }
                }
                if need_to_close {
                    println!("Closing connection on token={:?}", token);
                    conns.remove(token);
                }

            }
        }
    }
}
