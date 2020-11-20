//! Reference implementation of a mail service
//! simply delivering mail to server console log.
use std::fmt;

use crate::common::*;
use crate::mail::*;
//use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct DebugMailService {
    id: String,
}
impl DebugMailService {
    pub fn new(id: String) -> Self {
        Self { id: id }
    }
}
impl Default for DebugMailService {
    fn default() -> Self {
        Self {
            id: "samotop".to_owned(),
        }
    }
}
impl MailSetup for DebugMailService {
    fn setup(self, builder: &mut Builder) {
        builder.esmtp.insert(0, Box::new(self.clone()));
        builder.guard.insert(0, Box::new(self.clone()));
        builder.dispatch.insert(0, Box::new(self));
    }
}
impl EsmtpService for DebugMailService {
    fn prepare_session(&self, session: &mut SessionInfo) {
        info!("{}: I am {}", self.id, session.service_name);
    }
}

impl MailGuard for DebugMailService {
    fn add_recipient<'a, 'f>(
        &'a self,
        request: AddRecipientRequest,
    ) -> S2Fut<'f, AddRecipientResult>
    where
        'a: 'f,
    {
        info!(
            "{}: RCPT {} from {:?} (mailid: {:?}).",
            self.id, request.rcpt, request.transaction.mail, request.transaction.id
        );
        Box::pin(ready(AddRecipientResult::Inconclusive(request)))
    }
    fn start_mail<'a, 's, 'f>(
        &'a self,
        session: &'s SessionInfo,
        request: StartMailRequest,
    ) -> S2Fut<'f, StartMailResult>
    where
        'a: 'f,
        's: 'f,
    {
        info!(
            "{}: MAIL from {:?} (mailid: {:?}). {}",
            self.id, request.mail, request.id, session
        );
        Box::pin(ready(StartMailResult::Accepted(request)))
    }
}

impl MailDispatch for DebugMailService {
    fn send_mail<'a, 's, 'f>(
        &'a self,
        session: &'s SessionInfo,
        mut transaction: Transaction,
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
        } = transaction;
        info!(
            "Mail from {:?} for {} (mailid: {:?}). {}",
            mail.as_ref()
                .map(|m| m.path().to_string())
                .unwrap_or_else(|| "nobody".to_owned()),
            rcpts
                .iter()
                .fold(String::new(), |s, r| s + format!("{:?}, ", r.to_string())
                    .as_ref()),
            id,
            session
        );
        transaction.sink = transaction.sink.take().map(|inner| {
            Box::pin(DebugSink {
                id: id.clone(),
                inner,
            }) as Pin<Box<dyn MailDataSink>>
        });
        Box::pin(ready(Ok(transaction)))
    }
}

pub struct DebugSink {
    id: String,
    inner: Pin<Box<dyn MailDataSink>>,
}

impl Write for DebugSink {
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.inner.as_mut().poll_flush(cx)
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.inner.as_mut().poll_flush(cx) {
            Poll::Ready(Ok(())) => {
                info!("Mail complete: {}", self.id);
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(e)) => {
                info!("Mail failed: {} - {:?}", self.id, e);
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
                    "Mail data written: {} len {} {:?}",
                    self.id,
                    len,
                    String::from_utf8_lossy(&buf[..len])
                );
                Poll::Ready(Ok(len))
            }
            Poll::Ready(Err(e)) => {
                info!("Mail data failed: {} - {:?}", self.id, e);
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
    use futures_await_test::async_test;

    #[async_test]
    async fn test_setup() {
        let sess = SessionInfo::default();
        let tran = Transaction::default();
        let sut = DebugMailService::default();
        let _tran = sut.start_mail(&sess, tran).await;
    }
}
