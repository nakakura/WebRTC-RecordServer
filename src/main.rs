#![feature(drain_filter)]
extern crate futures;
extern crate tokio;
extern crate tokio_io;
extern crate bytes;

use futures::*;
use futures::sync::mpsc;
use tokio::net::{UdpSocket, UdpFramed};
use tokio_io::codec::BytesCodec;
use tokio::executor::current_thread::CurrentThread;
use bytes::Bytes;

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
    let sock = UdpSocket::bind(&addr).unwrap();
    let (a_sink, a_stream) = UdpFramed::new(sock, BytesCodec::new()).split();
    let task = a_stream.map_err(|_| ()).for_each(|x| {
        println!("recv {:?}", x);
        Ok(())
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
