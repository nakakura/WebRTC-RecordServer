use nom::*;
use futures::*;
use futures::sync::mpsc;
use futures_fs::*;
use tokio::executor::current_thread::CurrentThread;
use bytes::{Bytes, BytesMut};
use chrono::*;

use std::thread;
use std::collections::vec_deque::*;

type TimeStampAndBuf = (u32, Bytes);

pub struct RtpFileStream {
    reader: FsReadStream,
    data:  VecDeque<TimeStampAndBuf>,
    buf: BytesMut,
    offset: DateTime<Utc>,
    initial_ts: Option<u32>,
}

impl RtpFileStream {
    pub fn new(file_name: &'static str) -> Self {
        let fs = FsPool::default();
        let reader = fs.read(file_name, Default::default());
        RtpFileStream {
            reader: reader,
            data: VecDeque::new(),
            buf: BytesMut::new(),
            offset: Utc::now(),
            initial_ts: None,
        }
    }

    fn run(&self, time: DateTime<Utc>, task: task::Task) {
         thread::spawn(move || {
             let duration = time.signed_duration_since(Utc::now());
             if duration.num_milliseconds() > 0 {
                 thread::sleep(duration.to_std().unwrap());
             }

             task.notify();
        });
    }

    fn reload_data(&mut self) {
        if self.data.len() > 0 {
            return;
        }

        let data_opt = self.read_file();
        data_opt.map(|data| {
            self.buf.extend(data);
            let bytes = self.buf.take();
            let (data, rest) = parse(vec!(), &bytes);
            self.buf.extend(rest);
            self.data.extend(data.into_iter().map(|d| {
                let ts: u32 = get_rtp_ts(&d).unwrap().1;
                (ts, d)
            }));
        });
    }

    fn read_file(&mut self) -> Option<Bytes> {
        let poll = self.reader.poll().unwrap();
        if let Async::Ready(x) = poll {
            x
        } else {
            thread::sleep(::std::time::Duration::from_millis(1));
            self.read_file()
        }
    }

    fn calc_send_timing(&mut self, ts: u32) -> DateTime<Utc> {
        self.initial_ts = Some(self.initial_ts.unwrap_or(ts));
        let initial_ts: u64 = self.initial_ts.unwrap() as u64;
        let u32_max: u64 = ::std::u32::MAX as u64;
        let ts_diff: u64 = (ts as u64 + u32_max - initial_ts) % u32_max;
        let diff_ms: f64 = (ts_diff as f64) / 90f64;
        self.offset + Duration::milliseconds(diff_ms as i64)
    }
}

impl Stream for RtpFileStream {
    type Item = Bytes;
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.reload_data();

        if self.data.len() == 0 {
            Ok(Async::NotReady)
        } else {
            let ts = self.data.front().unwrap().0;
            let time = self.calc_send_timing(ts);

            if time < Utc::now() {
                let head = self.data.pop_front().unwrap();
                Ok(Async::Ready(Some(head.1)))
            } else {
                self.run(time, task::current());
                Ok(Async::NotReady)
            }
        }
    }
}

pub fn read_file(name: &'static str, current_thread: &mut CurrentThread) -> mpsc::Receiver<Bytes> {
    let (_tx, rx) = mpsc::channel::<Bytes>(1500);
    let stream = RtpFileStream::new(name);
    current_thread.spawn(stream.for_each(|_x| {
        //println!("output {:?}", x);
        Ok(())
    }));
    //let fs = FsPool::default();
    //let read = fs.read(name, Default::default());
    //let sink = RtpFileSink::new();
    //read.forward(sink).wait();
    /*
    let r = read.map_err(|_| ()).fold((tx, BytesMut::with_capacity(999999)), |mut sum, x| {
        sum.1.put(x);
        let r = parse(vec!(), &sum.1);
        let tx = sum.0.send(r.0).wait().unwrap();
        let mut buf = BytesMut::with_capacity(999999);
        buf.put(r.1);

        Ok((tx, buf))
    });

    */
    //current_thread.spawn(r);
    rx
}

fn parse(mut v: Vec<Bytes>, i: &[u8]) -> (Vec<Bytes>, &[u8]) {
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

fn get_rtp_ts<'a, 'b>(i: &'b [u8]) -> IResult<&'b [u8], u32> {
    do_parse!(i,
                _skip: be_u32
            >>  ts: be_u32
            >> (
                ts
            )
        )
}