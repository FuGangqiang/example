extern crate byteorder;
extern crate futures;
extern crate tokio_core;
extern crate tokio_proto;
extern crate tokio_service;

use std::io;
use std::str;
use std::net::SocketAddr;

use byteorder::{BigEndian, ByteOrder};
use futures::Future;
use futures::future::{self, BoxFuture};
use tokio_core::io::{Io, Framed, EasyBuf, Codec};
use tokio_proto::TcpServer;
use tokio_proto::multiplex::{RequestId, ServerProto};
use tokio_service::Service;

struct LineCodec;
struct LineProto;
struct EchoService;


/// # Frame Struct:
///
/// * request id: 4 byte
/// * payload: zero or more bytes
/// * linefeed: `\n`
///
/// +-- request id --+------- frame payload --------+
/// |                |                              |
/// |   \x00000001   | This is the frame payload \n |
/// |                |                              |
/// +----------------+------------------------------+
///
impl Codec for LineCodec {
    type In = (RequestId, String);
    type Out = (RequestId, String);

    fn decode(&mut self, buf: &mut EasyBuf) -> Result<Option<(RequestId, String)>, io::Error>
    {
        if buf.len() < 5 {
            return Ok(None);  // We don't yet have a full message
        }

        let newline = buf.as_ref()[4..].iter().position(|b| *b == b'\n');
        if let Some(n) = newline {
            let line = buf.drain_to(n + 4);
            buf.drain_to(1);
            let id = BigEndian::read_u32(&line.as_ref()[0..4]);
            return match str::from_utf8(&line.as_ref()[4..]) {
                Ok(s) => Ok(Some((id as RequestId, s.to_string()))),
                Err(_) => Err(io::Error::new(io::ErrorKind::Other, "invalid string")),
            }
        }

        Ok(None)  // We don't yet have a full message
    }

    fn encode(&mut self, msg: (RequestId, String), buf: &mut Vec<u8>) -> io::Result<()>
    {
        let (id, msg) = msg;
        let mut encoded_id = [0; 4];

        BigEndian::write_u32(&mut encoded_id, id as u32);
        buf.extend(&encoded_id);
        buf.extend(msg.as_bytes());
        buf.push(b'\n');

        Ok(())
    }
}


impl<T: Io + 'static> ServerProto<T> for LineProto {
    type Request = String;
    type Response = String;

    // `Framed<T, LineCodec>` is the return value of `io.framed(LineCodec)`
    type Transport = Framed<T, LineCodec>;
    type BindTransport = Result<Self::Transport, io::Error>;

    fn bind_transport(&self, io: T) -> Self::BindTransport {
        Ok(io.framed(LineCodec))
    }
}


impl Service for EchoService {
    type Request = String;
    type Response = String;
    type Error = io::Error;
    type Future = BoxFuture<Self::Response, Self::Error>;

    fn call(&self, req: Self::Request) -> Self::Future {
        future::ok(req).boxed()
    }
}


fn main() {
    let mut args = ::std::env::args();
    let cmd = args.next().unwrap();
    let port = args.next().expect(&format!("Usage: {} [port]", cmd));
    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().expect("argument format error: port");
    let server = TcpServer::new(LineProto, addr);
    server.serve(|| Ok(EchoService));
}
