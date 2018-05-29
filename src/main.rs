#![feature(drain_filter)]
extern crate futures;
extern crate futures_fs;
extern crate tokio;
extern crate tokio_io;
extern crate bytes;
#[macro_use]
extern crate nom;

mod file;

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

    let mut current_thread = CurrentThread::new();

    let rx = file::read_file("./dump", &mut current_thread);
    socket(addr1, &mut current_thread, rx);

    let _x = current_thread.run();
}

fn socket(addr: SocketAddr, current_thread: &mut CurrentThread, rx: mpsc::Receiver<Bytes>) {
    let sock = UdpSocket::bind(&addr).unwrap();
    let (a_sink, a_stream) = UdpFramed::new(sock, BytesCodec::new()).split();

    let fs = FsPool::default();
    let write = fs.write("./dump2", Default::default());

    let stream = a_stream.map(|(data, _addr)| {
        let len = data.len();
        let mut buf = BytesMut::with_capacity(len + 2);
        buf.put_u16_be(len as u16);
        buf.put(data);
        buf.freeze()
    }).forward(write);

    let sender = a_sink.sink_map_err(|e| {
        eprintln!("err {:?}", e);
    }).send_all(rx.map(move |x| (x, addr))).then(|_| Ok(()));

    current_thread.spawn({
        sender.join(stream)
            .map(|_| ())
            .map_err(|e| println!("error = {:?}", e))
    });
}
