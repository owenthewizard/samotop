use std::io;
use futures::{Sink, Stream, Poll, StartSend, Async};
use tokio_proto::streaming::pipeline::Transport;

pub type Error = io::Error;

pub struct InitFrameTransport<TT, TRequest>
where
    TRequest: 'static,
{
    initframe: Option<TRequest>,
    upstream: TT,
}

impl<TT, TRequest> InitFrameTransport<TT, TRequest> {
    pub fn new(upstream: TT, initframe: TRequest) -> Self {
        Self {
            upstream,
            initframe: Some(initframe),
        }
    }
}

impl<TT, TRequest> Stream for InitFrameTransport<TT, TRequest>
where
    TT: 'static,
    TT: Stream<
        Error = Error,
        Item = TRequest,
    >,
{
    type Error = Error;
    type Item = TRequest;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        trace!("poll");
        match self.initframe.take() {
            Some(frame) => {
                trace!("transport initializing");
                Ok(Async::Ready(Some(frame)))
            }
            None => self.upstream.poll(),
        }
    }
}

impl<TT, TRequest, TResponse> Sink for InitFrameTransport<TT, TRequest>
where
    TT: 'static,
    TT: Sink<
        SinkError = Error,
        SinkItem = TResponse,
    >,
{
    type SinkError = Error;
    type SinkItem = TResponse;

    fn start_send(&mut self, request: Self::SinkItem) -> StartSend<Self::SinkItem, io::Error> {
        self.upstream.start_send(request)
    }

    fn poll_complete(&mut self) -> Poll<(), io::Error> {
        self.upstream.poll_complete()
    }

    fn close(&mut self) -> Poll<(), io::Error> {
        self.upstream.close()
    }
}

impl<TT, TRequest, TResponse> Transport for InitFrameTransport< TT, TRequest>
where
    TT: 'static,
    TT: Stream< Error = Error, Item = TRequest>,
    TT: Sink< SinkError = Error, SinkItem = TResponse>
{
}





#[cfg(test)]
mod tests {
    use std::io;
    use std::error::Error;
    use std::sync::mpsc::{Sender, Receiver, channel};
    use futures::{Sink, Stream, Async, AsyncSink, Poll, StartSend};
    use tokio_proto::streaming::pipeline::Transport;
    use super::InitFrameTransport;

    type Sut = InitFrameTransport<MockTransport, u8>;

    #[test]
    fn new_creates() {

        let (upstream, _tx_cmd, _rx_rpl) = MockTransport::setup();

        let _sut = Sut::new(upstream, 1);
    }

    #[test]
    fn poll_pops_initframe_from_stream() {

        let (upstream, tx_cmd, _rx_rpl) = MockTransport::setup();

        tx_cmd.send(Ok(Async::Ready(Some(2)))).unwrap();

        let mut sut = Sut::new(upstream, 1);

        match sut.poll() {
            Ok(Async::Ready(Some(1))) => (),
            otherwise => panic!(otherwise),
        }
    }

    #[test]
    fn poll_pops_nextframe_from_stream() {

        let (upstream, tx_cmd, _rx_rpl) = MockTransport::setup();

        tx_cmd.send(Ok(Async::Ready(Some(2)))).unwrap();

        let mut sut = Sut::new(upstream, 1);

        sut.poll().unwrap();

        match sut.poll() {
            Ok(Async::Ready(Some(2))) => (),
            otherwise => panic!(otherwise),
        }
    }

    #[test]
    fn sink_passes_frame() {

        let (upstream, _tx_cmd, rx_rpl) = MockTransport::setup();

        let mut sut = Sut::new(upstream, 1);

        sut.start_send(3).unwrap();
        sut.poll_complete().unwrap();
        sut.close().unwrap();

        match rx_rpl.recv() {
            Ok(3) => (),
            otherwise => panic!(otherwise),
        }
    }




    pub struct MockTransport {
        pub stream: Receiver<Result<Async<Option<u8>>, io::Error>>,
        pub sink: Sender<i32>,
    }
    impl MockTransport {
        pub fn setup() -> (Self, Sender<Result<Async<Option<u8>>, io::Error>>, Receiver<i32>) {

            let (tx_cmd, rx_cmd): (Sender<Result<Async<Option<u8>>, io::Error>>,
                                   Receiver<Result<Async<Option<u8>>, io::Error>>) = channel();
            let (tx_rpl, rx_rpl): (Sender<i32>, Receiver<i32>) = channel();
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
        type Item = u8;
        type Error = io::Error;
        fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
            match self.stream.recv() {
                Err(e) => Err(io::Error::new(io::ErrorKind::Other, e.description())),
                Ok(f) => f,
            }
        }
    }
    impl Sink for MockTransport {
        type SinkItem = i32;
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
}
