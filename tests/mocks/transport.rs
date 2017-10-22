extern crate samotop;
extern crate tokio_proto;
extern crate futures;
extern crate bytes;

use std::io;
use std::error::Error;
use std::sync::mpsc::{Sender, Receiver, channel};
use futures::{Stream, Sink, Async, AsyncSink, Poll, StartSend};
use tokio_proto::streaming::pipeline::Transport;
use samotop::protocol::{CmdFrame, RplFrame};

pub struct MockTransport {
    pub stream: Receiver<Result<Async<Option<CmdFrame>>, io::Error>>,
    pub sink: Sender<RplFrame>,
}
impl MockTransport {
    pub fn setup()
        -> (Self, Sender<Result<Async<Option<CmdFrame>>, io::Error>>, Receiver<RplFrame>)
    {

        let (tx_cmd, rx_cmd): (Sender<Result<Async<Option<CmdFrame>>, io::Error>>,
                               Receiver<Result<Async<Option<CmdFrame>>, io::Error>>) = channel();
        let (tx_rpl, rx_rpl): (Sender<RplFrame>, Receiver<RplFrame>) = channel();
        (
            Self {
                sink: tx_rpl,
                stream: rx_cmd,
            },
            tx_cmd,
            rx_rpl,
        )
    }
}
impl Stream for MockTransport {
    type Item = CmdFrame;
    type Error = io::Error;
    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        match self.stream.recv() {
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e.description())),
            Ok(f) => f,
        }
    }
}
impl Sink for MockTransport {
    type SinkItem = RplFrame;
    type SinkError = io::Error;

    fn start_send(&mut self, request: Self::SinkItem) -> StartSend<Self::SinkItem, io::Error> {
        match self.sink.send(request) {
            Ok(_) => Ok(AsyncSink::Ready),
            Err(_) => Err(io::ErrorKind::Other.into()),
        }
    }

    fn poll_complete(&mut self) -> Poll<(), io::Error> {
        Ok(Async::Ready(()))
    }

    fn close(&mut self) -> Poll<(), io::Error> {
        Ok(Async::Ready(()))
    }
}
impl Transport for MockTransport {}
