use futures::prelude::*;
use futures::stream;
use futures::{Async, AsyncSink};
use std::io;
use std::iter;
use tokio;

#[test]
fn run() {
    let x = X;
    let (sink, stream) = S { i: 4, j: 0 }.split();

    let task = stream
        .map(|i| i+1)
        .map(move |i| x.make_ones(i))
        .flatten()
        .inspect(|i| println!("i: {:?}", i))
        //.map_err(|e| println!("stream error {:?}", e))
        .forward(sink)
        .map(|_| println!("it's over"))
        .map_err(|e| println!("sink error {:?}", e));

    tokio::run(task)
}

struct X;
impl X {
    fn make_ones(&self, n: u8) -> impl Stream<Item = u8, Error = io::Error> {
        stream::iter_ok(iter::repeat(1).take(n.into()))
    }
}

#[derive(Debug)]
struct S {
    pub i: u8,
    pub j: u8,
}

impl Sink for S {
    type SinkItem = u8;
    type SinkError = io::Error;

    fn start_send(&mut self, n: Self::SinkItem) -> io::Result<AsyncSink<Self::SinkItem>> {
        self.j += n;
        println!("j: {:?}", self.j);
        Ok(AsyncSink::Ready)
    }

    fn poll_complete(&mut self) -> Result<Async<()>, io::Error> {
        Ok(Async::Ready(()))
    }
}

impl Stream for S {
    type Item = u8;
    type Error = io::Error;

    fn poll(&mut self) -> io::Result<Async<Option<Self::Item>>> {
        if self.i == 0 {
            Ok(Async::Ready(None))
        } else {
            self.i -= 1;
            Ok(Async::Ready(Some(self.i)))
        }
    }
}
