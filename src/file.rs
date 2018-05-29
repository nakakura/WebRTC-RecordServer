use nom::*;
use futures::*;
use futures::sync::mpsc;
use futures_fs::FsPool;
use tokio::executor::current_thread::CurrentThread;
use bytes::{Bytes, BytesMut, BufMut};

pub fn read_file(name: &'static str, current_thread: &mut CurrentThread) -> mpsc::Receiver<Bytes> {
    let (tx, rx) = mpsc::channel::<Bytes>(1500);
    let tx = tx.with_flat_map(|vec: Vec<Bytes>| {
        stream::iter_ok(vec.into_iter().map(|x| x))
    });

    let fs = FsPool::default();
    let read = fs.read(name, Default::default());
    let r = read.map_err(|_| ()).fold((tx, BytesMut::with_capacity(999999)), |mut sum, x| {
        sum.1.put(x);
        let r = parse(vec!(), &sum.1);
        let tx = sum.0.send(r.0).wait().unwrap();
        let mut buf = BytesMut::with_capacity(999999);
        buf.put(r.1);

        Ok((tx, buf))
    });

    current_thread.spawn(r.map(|_| ()).map_err(|_| ()));
    rx
}

fn parse<'a>(mut v: Vec<Bytes>, i: &'a [u8]) -> (Vec<Bytes>, &'a [u8]) {
    let x = extract(i);
    if x.is_done() {
        let x = x.unwrap().1;
        v.push(Bytes::from(x.1));
        parse(v, x.2)
    } else if x.is_incomplete() {
        (v, i)
    } else {
        println!("err");
        unreachable!();
    }
}

fn extract(i: &[u8]) -> IResult<&[u8], (u16, &[u8], &[u8])> {
    let total_len = i.len();
    do_parse!(i,
                data_len: be_u16
            >>  payload: take!(data_len)
            >>  rest: take!(total_len - data_len as usize - 2)
            >> (
                data_len, payload, rest
            )
        )
}

