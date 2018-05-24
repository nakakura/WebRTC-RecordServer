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

    socket(&addr1, &mut current_thread);

    let _x = current_thread.run();
}

fn socket(addr: &SocketAddr, current_thread: &mut CurrentThread) {
    let fs = FsPool::default();
    let sock = UdpSocket::bind(&addr).unwrap();
    let (a_sink, a_stream) = UdpFramed::new(sock, BytesCodec::new()).split();
    let write = fs.write("./dump2", Default::default());
    let read = fs.read("./dump", Default::default());
    let (tx, rx) = mpsc::channel::<Bytes>(1500);
    let mut buf = BytesMut::with_capacity(999999);

    let r = read.map_err(|_| ()).fold((tx, buf), |mut sum: (mpsc::Sender<Bytes>, BytesMut), x| {
        use nom::IResult::Done;
        sum.1.put(x);
        let r = file::parse(vec!(), &sum.1);
        let mut tx = sum.0;
        for i in 0..r.0.len() {
            tx = tx.send(Bytes::from(r.0[i])).wait().unwrap();
        }

        let mut buf = BytesMut::with_capacity(999999);
        buf.put(r.1);

        Ok((tx, buf))
    }).map(|x| ());

    let task = a_stream.map_err(|_| ()).fold(write, |w, (data, _addr)| {
        let len = data.len();
        let mut buf = BytesMut::with_capacity(len + 2);
        buf.put_u16_be(len as u16);
        buf.put(data);
        let w = w.send(buf.freeze()).wait().unwrap();
        Ok(w)
    });

    let addr1: SocketAddr = format!("{}:{}", "127.0.0.1", 10001).parse().unwrap();

    let sender = a_sink.sink_map_err(|e| {
        eprintln!("err {:?}", e);
    }).send_all(rx.map(move |x| (x, addr1))).then(|_| Ok(()));

    current_thread.spawn(r.map_err(|_| ()));
    current_thread.spawn({
        sender.join(task)
            .map(|_| ())
            .map_err(|e| println!("error = {:?}", e))
    });
}
