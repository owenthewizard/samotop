//! Reference implementation of a mail service
//! simply delivering mail to server console log.
use crate::{
    common::*,
    io::tls::MayBeTls,
    mail::*,
    smtp::{SessionService, SmtpContext, SmtpSession},
};
use std::fmt;
//use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct DebugService {
    id: String,
}
impl DebugService {
    pub fn new(id: String) -> Self {
        Self { id }
    }
}
impl Default for DebugService {
    fn default() -> Self {
        Self {
            id: time_based_id(),
        }
    }
}
impl<T> MailSetup<T> for DebugService
where
    T: AcceptsSessionService + AcceptsGuard + AcceptsDispatch,
{
    fn setup(self, config: &mut T) {
        config.add_last_session_service(self.clone());
        config.add_last_guard(self.clone());
        config.add_last_dispatch(self);
    }
}
impl SessionService for DebugService {
    fn prepare_session<'a, 'i, 's, 'f>(
        &'a self,
        _io: &'i mut Box<dyn MayBeTls>,
        state: &'s mut SmtpContext,
    ) -> S1Fut<'f, ()>
    where
        'a: 'f,
        'i: 'f,
        's: 'f,
    {
        info!(
            "{}: I am {}! Preparing {}",
            self.id, state.session.service_name, state.session.connection
        );
        Box::pin(ready(()))
    }
}

impl MailGuard for DebugService {
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
            self.id, rcpt.address, session.transaction.mail, session.transaction.id
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
            self.id, session.transaction.mail, session.transaction.id, session
        );
        Box::pin(ready(StartMailResult::Accepted))
    }
}

impl MailDispatch for DebugService {
    fn open_mail_body<'a, 's, 'f>(
        &'a self,
        session: &'s mut SmtpSession,
    ) -> S1Fut<'f, DispatchResult>
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
            "Mail from {:?} for {} (mailid: {:?}). {}",
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
                id: id.clone(),
                inner,
            }) as Pin<Box<dyn MailDataSink>>
        });
        Box::pin(ready(Ok(())))
    }
}

pub struct DebugSink {
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

    #[test]
    fn test_setup() {
        async_std::task::block_on(async move {
            let mut sess = SmtpSession::default();
            let sut = DebugService::default();
            let tran = sut.start_mail(&mut sess).await;
            assert_eq!(tran, StartMailResult::Accepted)
        })
    }
}
