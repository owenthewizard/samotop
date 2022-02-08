//! Reference implementation of a mail service
//! simply delivering mail to server console log.
use crate::{
    config::{ServerContext, Setup},
    common::*,
    io::{ConnectionInfo, Handler, HandlerService, Session},
    mail::*,
    smtp::SmtpSession,
};
use std::fmt;

/// Produce info logs on important e-mail and SMTP events.
///
/// The logger will use session service name to mark the logs.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct SessionLogger;

pub use SessionLogger as DebugService;

impl Setup for SessionLogger {
    fn setup(&self, ctx: &mut ServerContext) {
        ctx.store.add::<HandlerService>(Arc::new(self.clone()));
    }
}
impl Handler for SessionLogger {
    fn handle<'s, 'a, 'f>(&'s self, session: &'a mut Session) -> S2Fut<'f, Result<()>>
    where
        's: 'f,
        'a: 'f,
    {
        info!("Preparing {:?}", session.store.get_ref::<ConnectionInfo>());
        session
            .store
            .add::<MailGuardService>(Arc::new(SessionLogger));
        session
            .store
            .add::<MailDispatchService>(Arc::new(SessionLogger));
        Box::pin(ready(Ok(())))
    }
}

impl MailGuard for SessionLogger {
    fn add_recipient<'a, 's, 'f>(
        &'a self,
        session: &'s mut SmtpSession,
        rcpt: Recipient,
    ) -> S2Fut<'f, AddRecipientResult>
    where
        'a: 'f,
        's: 'f,
    {
        info!(
            "{}: RCPT {} from {:?} (mailid: {:?}).",
            session.service_name, rcpt.address, session.transaction.mail, session.transaction.id
        );
        Box::pin(ready(AddRecipientResult::Inconclusive(rcpt)))
    }
    fn start_mail<'a, 's, 'f>(&'a self, session: &'s mut SmtpSession) -> S2Fut<'f, StartMailResult>
    where
        'a: 'f,
        's: 'f,
    {
        info!(
            "{}: MAIL from {:?} (mailid: {:?}). {}",
            session.service_name, session.transaction.mail, session.transaction.id, session
        );
        Box::pin(ready(StartMailResult::Accepted))
    }
}

impl MailDispatch for SessionLogger {
    fn open_mail_body<'a, 's, 'f>(
        &'a self,
        session: &'s mut SmtpSession,
    ) -> S2Fut<'f, DispatchResult>
    where
        'a: 'f,
        's: 'f,
    {
        let Transaction {
            ref mail,
            ref id,
            ref rcpts,
            ..
        } = session.transaction;
        info!(
            "{}: Mail from {:?} for {} (mailid: {:?}). {}",
            session.service_name,
            mail.as_ref()
                .map(|m| m.sender().to_string())
                .unwrap_or_else(|| "nobody".to_owned()),
            rcpts.iter().fold(String::new(), |s, r| s + format!(
                "{:?}, ",
                r.address.to_string()
            )
            .as_ref()),
            id,
            session
        );
        session.transaction.sink = session.transaction.sink.take().map(|inner| {
            Box::pin(DebugSink {
                id: format!("{}: {}", session.service_name, id.clone()),
                inner,
            }) as Pin<Box<dyn MailDataSink>>
        });
        Box::pin(ready(Ok(())))
    }
}

struct DebugSink {
    id: String,
    inner: Pin<Box<dyn MailDataSink>>,
}

impl io::Write for DebugSink {
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.inner.as_mut().poll_flush(cx)
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.inner.as_mut().poll_flush(cx) {
            Poll::Ready(Ok(())) => {
                info!("{}: Mail complete", self.id);
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(e)) => {
                info!("{}: Mail failed: {:?}", self.id, e);
                Poll::Ready(Ok(()))
            }
            Poll::Pending => Poll::Pending,
        }
    }
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.inner.as_mut().poll_write(cx, buf) {
            Poll::Ready(Ok(len)) => {
                debug!(
                    "{}: Mail data written: len {} {:?}",
                    self.id,
                    len,
                    String::from_utf8_lossy(&buf[..len])
                );
                Poll::Ready(Ok(len))
            }
            Poll::Ready(Err(e)) => {
                info!("{}: Mail data failed: {:?}", self.id, e);
                Poll::Ready(Err(e))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl fmt::Debug for DebugSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DebugSink")
            .field("id", &self.id)
            .field("inner", &"*")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_setup() {
        let mut sess = SmtpSession::default();
        let sut = SessionLogger;
        let tran = sut.start_mail(&mut sess).await;
        assert_eq!(tran, StartMailResult::Accepted)
    }
}
