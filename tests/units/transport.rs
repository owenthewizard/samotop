use env_logger;
use std::io;
use std::error::Error;
use std::fmt::Debug;
use std::sync::mpsc::{Sender, Receiver, channel};
use futures::{Sink, Stream, Async, AsyncSink, Poll, StartSend};
use tokio_proto::streaming::pipeline::Transport;
use samotop::model::request::*;
use samotop::model::response::SmtpReply;
use samotop::protocol::stateful::ActTransport;

type Sut = ActTransport<MockTransport<SmtpInput, SmtpReply>>;

#[test]
fn new_creates() {

    let (upstream, _tx_inp, _rx_rpl) = MockTransport::setup();

    let _sut = Sut::new(upstream);
}

#[test]
fn streams_input() {

    env_logger::init().unwrap();

    let (upstream, tx_inp, _rx_rpl) = MockTransport::setup();

    let mut sut = Sut::new(upstream);

    tx_inp
        .send(Ok(Async::Ready(Some(SmtpInput::Connect(SmtpConnection {
            local_name: "unit".to_string(),
            peer_addr: None,
            local_addr: None,
        })))))
        .unwrap();

    tx_inp.send(Ok(Async::NotReady)).unwrap();

    match sut.poll() {
        Ok(Async::NotReady) => (),
        _ => panic!(""),
    }
}



pub struct MockTransport<TIn, TOut> {
    pub stream: Receiver<Result<Async<Option<TIn>>, io::Error>>,
    pub sink: Sender<TOut>,
}
impl<TIn, TOut> MockTransport<TIn, TOut> {
    pub fn setup() -> (Self, Sender<Result<Async<Option<TIn>>, io::Error>>, Receiver<TOut>) {

        let (tx_cmd, rx_cmd): (Sender<Result<Async<Option<TIn>>, io::Error>>,
                               Receiver<Result<Async<Option<TIn>>, io::Error>>) = channel();
        let (tx_rpl, rx_rpl): (Sender<TOut>, Receiver<TOut>) = channel();
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
impl<TIn, TOut> Stream for MockTransport<TIn, TOut>
where
    TIn: Debug,
{
    type Item = TIn;
    type Error = io::Error;
    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        let inp = self.stream.recv();
        trace!("{:?}", inp);
        match inp {
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e.description())),
            Ok(f) => f,
        }
    }
}
impl<TIn, TOut> Sink for MockTransport<TIn, TOut> {
    type SinkItem = TOut;
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
impl<TIn, TOut> Transport for MockTransport<TIn, TOut>
where
    TIn: Debug,
    TIn: 'static,
    TOut: 'static,
{
}
