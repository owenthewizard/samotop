use crate::{
    common::*,
    mail::{AcceptsDispatch, DispatchResult, MailDispatch, MailSetup},
    smtp::SmtpSession,
};

#[derive(Debug)]
pub struct NullDispatch;

impl MailDispatch for NullDispatch {
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
impl<T: AcceptsDispatch> MailSetup<T> for NullDispatch {
    fn setup(self, config: &mut T) {
        config.add_last_dispatch(self)
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
