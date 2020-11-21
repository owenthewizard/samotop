//! The stub transport only logs message envelope and drops the content. It can be useful for
//! testing purposes.
//!

mod error;

pub use self::error::*;
use crate::{
    stub::error::{Error, StubResult},
    Envelope, MailDataStream, SyncFuture, Transport,
};
use async_std::io::Write;
use futures::future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// This transport logs the message envelope and returns the given response
#[derive(Debug)]
pub struct StubTransport {
    response: StubResult,
}

impl StubTransport {
    /// Creates a new transport that always returns the given response
    pub fn new(response: StubResult) -> StubTransport {
        StubTransport { response }
    }

    /// Creates a new transport that always returns a success response
    pub fn new_positive() -> StubTransport {
        StubTransport { response: Ok(()) }
    }
}

impl Transport for StubTransport {
    type DataStream = StubStream;
    fn send_stream<'life1, 'async_trait>(
        &'life1 self,
        envelope: Envelope,
    ) -> SyncFuture<Result<StubStream, Error>>
    where
        'life1: 'async_trait,
    {
        info!(
            "{}: from=<{}> to=<{:?}>",
            envelope.message_id(),
            match envelope.from() {
                Some(address) => address.to_string(),
                None => "".to_string(),
            },
            envelope.to()
        );
        Box::pin(future::ready(Ok(StubStream {
            response: self.response.clone(),
        })))
    }
}

#[derive(Debug)]
pub struct StubStream {
    response: StubResult,
}

impl MailDataStream for StubStream {
    type Output = ();
    type Error = Error;
    fn result(&self) -> StubResult {
        info!("Done: {:?}", self.response);
        self.response.clone()
    }
}

impl Write for StubStream {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        info!("Writing {} bytes", buf.len());
        Poll::Ready(Ok(buf.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        info!("Flushing");
        Poll::Ready(Ok(()))
    }
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        info!("Closing");
        Poll::Ready(Ok(()))
    }
}
