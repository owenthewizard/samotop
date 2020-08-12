use crate::model::io::*;
use crate::model::smtp::SmtpReply;
use crate::model::{Error, Result};
use crate::service::mail::*;
use crate::service::session::*;
use futures::prelude::*;
use pin_project::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone)]
pub struct DummySessionService<S> {
    mail_service: S,
}

impl<S> DummySessionService<S> {
    pub fn new(mail_service: S) -> Self {
        Self { mail_service }
    }
}

impl<S> SessionService for DummySessionService<S>
where
    S: Send,
    S: Clone,
    S: NamedService,
{
    type Handler = DummySessionHandler;
    fn start(&self) -> Self::Handler {
        let name = self.mail_service.name();
        DummySessionHandler::new(name)
    }
}

#[pin_project(project=HandlerProjection)]
#[must_use = "streams and sinks do nothing unless polled"]
pub struct DummySessionHandler {
    name: String,
    state: u8,
    closed: bool,
}

impl DummySessionHandler {
    pub fn new(name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            state: 0,
            closed: false,
        }
    }
}

impl Sink<ReadControl> for DummySessionHandler {
    type Error = Error;
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        trace!("Polling flush");
        assert!(self.closed == false, "called poll_flush() on closed sink");
        self.poll_ready(cx)
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        trace!("Polling close");
        assert!(self.closed == false, "called poll_close() on closed sink");
        let res = self.as_mut().poll_flush(cx);
        self.closed = true;
        res
    }
    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        trace!("Polling ready");
        assert!(self.closed == false, "called poll_ready() on closed sink");
        Poll::Ready(Ok(()))
    }

    fn start_send(mut self: Pin<&mut Self>, item: ReadControl) -> Result<()> {
        trace!("Sink item: {:?}", item);
        assert!(self.closed == false, "called start_send() on closed sink");
        self.state += 1;
        Ok(())
    }
}
impl Stream for DummySessionHandler {
    type Item = Result<WriteControl>;
    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        trace!("Polling next session answer. pending {}", self.state);

        if self.state != 0 {
            self.state -= 1;
            Poll::Ready(Some(Ok(WriteControl::Reply(SmtpReply::ServiceReadyInfo(
                self.name.clone(),
            )))))
        } else if self.closed {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}
