use crate::common::*;
use crate::session::*;
use crate::smtp::{ReadControl, SmtpReply, WriteControl};
use futures::prelude::*;
use pin_project::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone)]
pub struct DummySessionService {
    name: String,
}

impl DummySessionService {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl<TIn> SessionService<TIn> for DummySessionService
where
    TIn: Stream<Item = Result<ReadControl>> + Unpin + Send + Sync + 'static,
{
    fn start(&self, input: TIn) -> SessionStream {
        Box::new(DummySessionHandler::new(self.name.clone(), input))
    }
}

#[pin_project(project=HandlerProjection)]
#[must_use = "streams and sinks do nothing unless polled"]
pub struct DummySessionHandler<TIn> {
    name: String,
    state: u8,
    closed: bool,
    #[pin]
    input: TIn,
}

impl<TIn> DummySessionHandler<TIn> {
    pub fn new(name: impl ToString, input: TIn) -> Self {
        Self {
            name: name.to_string(),
            state: 0,
            closed: false,
            input,
        }
    }
}

impl<TIn> Stream for DummySessionHandler<TIn>
where
    TIn: Stream<Item = Result<ReadControl>>,
{
    type Item = Result<WriteControl>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        trace!("Polling next response. Done {}", self.state);
        let proj = self.as_mut().project();

        if ready!(proj.input.poll_next(cx)?).is_none() {
            *proj.closed = true;
        }

        let result = if *proj.closed {
            Poll::Ready(None)
        } else {
            match *proj.state {
                0 => Poll::Ready(Some(Ok(WriteControl::Reply(SmtpReply::ServiceReadyInfo(
                    proj.name.clone(),
                ))))),
                _ => Poll::Ready(Some(Ok(WriteControl::Reply(
                    SmtpReply::ServiceNotAvailableError(proj.name.clone()),
                )))),
            }
        };

        *proj.state += 1;
        result
    }
}
