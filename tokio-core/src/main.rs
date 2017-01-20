extern crate futures;
extern crate tokio_core;

use std::net::SocketAddr;

use futures::Future;
use futures::stream::Stream;
use tokio_core::io::{copy, Io};
use tokio_core::net::TcpListener;
use tokio_core::reactor::Core;


fn main() {
    let mut args = ::std::env::args();
    let cmd = args.next().unwrap();
    let port = args.next().expect(&format!("Usage: {} [port]", cmd));
    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().expect("argument format error: port");

    let mut core = Core::new().unwrap();  // the Event Loop
    let handle = core.handle();

    let socket = TcpListener::bind(&addr, &handle).unwrap();
    let done = socket.incoming().for_each(|(socket, addr)| {
        let pair = futures::lazy(|| Ok(socket.split()));
        let amt = pair.and_then(|(reader, _)| copy(reader, ::std::io::stdout()));
        println!("Accepted connection: {}", addr);
        handle.spawn(amt.then(move |_| {
            println!("Closing connection on {}", addr);
            Ok(())
        }));
        Ok(())
    });

    // Start Event Loop
    core.run(done).unwrap();
}
