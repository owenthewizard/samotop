use super::{DispatchResult, MailDispatch, MailSetup};
use crate::{
    common::*,
    mail::AcceptsDispatch,
    smtp::{SessionInfo, Transaction},
};

#[derive(Debug)]
pub struct NullDispatch;

impl MailDispatch for NullDispatch {
    fn send_mail<'a, 's, 'f>(
        &'a self,
        _session: &'s SessionInfo,
        mut transaction: Transaction,
    ) -> crate::common::S1Fut<'f, DispatchResult>
    where
        'a: 'f,
        's: 'f,
    {
        if transaction.sink.is_none() {
            transaction.sink = Some(Box::pin(NullSink))
        }
        Box::pin(ready(Ok(transaction)))
    }
}
impl<T: AcceptsDispatch> MailSetup<T> for NullDispatch {
    fn setup(self, config: &mut T) {
        config.add_dispatch(self)
    }
}

struct NullSink;

impl Write for NullSink {
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
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
