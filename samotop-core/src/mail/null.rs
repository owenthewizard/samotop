use crate::{
    common::*,
    mail::{DispatchResult, MailDispatch, MailSetup},
    smtp::SmtpSession,
};

use super::Configuration;

/// Accept all calls, but do nothing.
/// Combine this with the `SessionLogger` for a light-weight debugging server.
#[derive(Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct NullDispatch;

impl MailSetup for NullDispatch {
    /// Add a null dispatch
    fn setup(self, config: &mut Configuration) {
        config.add_last_dispatch(self)
    }
}

impl MailDispatch for NullDispatch {
    /// If no sink is present, add null sink, accepting all data, doing nothing
    fn open_mail_body<'a, 's, 'f>(
        &'a self,
        session: &'s mut SmtpSession,
    ) -> S1Fut<'f, DispatchResult>
    where
        'a: 'f,
        's: 'f,
    {
        if session.transaction.sink.is_none() {
            session.transaction.sink = Some(Box::pin(NullSink))
        }
        Box::pin(ready(Ok(())))
    }
}

struct NullSink;

impl io::Write for NullSink {
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Ok(buf.len()))
    }
}
