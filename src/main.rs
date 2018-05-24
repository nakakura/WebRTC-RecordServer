#![feature(drain_filter)]
extern crate futures;
extern crate futures_fs;
extern crate tokio;
extern crate tokio_io;
extern crate bytes;
#[macro_use]
extern crate nom;
extern crate either;

use futures::*;
use futures::sync::mpsc;
use futures_fs::FsPool;
use tokio::net::{UdpSocket, UdpFramed};
use tokio_io::codec::BytesCodec;
use tokio::executor::current_thread::CurrentThread;
use bytes::{Bytes, BytesMut, BufMut};

use std::net::SocketAddr;

fn main() {
    let target_addr_str = "0.0.0.0";
    let addr1: SocketAddr = format!("{}:{}", &target_addr_str, 10000).parse().unwrap();

    let x1 = mpsc::channel::<(Bytes, SocketAddr)>(5000);

    let mut current_thread = CurrentThread::new();

    socket(&addr1, x1.1, &mut current_thread);

    let _x = current_thread.run();
}

fn socket(addr: &SocketAddr, stream: mpsc::Receiver<(Bytes, SocketAddr)>, current_thread: &mut CurrentThread) {
    let fs = FsPool::default();
    let sock = UdpSocket::bind(&addr).unwrap();
    let (a_sink, a_stream) = UdpFramed::new(sock, BytesCodec::new()).split();
    let write = fs.write("./dump", Default::default());

    let task = a_stream.map_err(|_| ()).fold(write, |w, (data, _addr)| {
        let len = data.len();
        let mut buf = BytesMut::with_capacity(len + 2);
        buf.put_u16_be(len as u16);
        buf.put(data);
        let w = w.send(buf.freeze()).wait().unwrap();
        Ok(w)
    });

    let sender = a_sink.sink_map_err(|e| {
        eprintln!("err {:?}", e);
    }).send_all(stream).then(|_| Ok(()));

    current_thread.spawn({
        sender.join(task)
            .map(|_| ())
            .map_err(|e| println!("error = {:?}", e))
    });
}
