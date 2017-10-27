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
