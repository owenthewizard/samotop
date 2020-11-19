//! Reference implementation of a mail service
//! simply delivering mail to server console log.
use crate::common::*;
use crate::mail::*;
//use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct DefaultMailService {
    name: String,
}
impl DefaultMailService {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}
impl Default for DefaultMailService {
    fn default() -> Self {
        Self {
            name: "samotop".to_owned(),
        }
    }
}
impl MailSetup for DefaultMailService {
    fn setup(self, builder: &mut Builder) {
        builder.esmtp.insert(0, Box::new(self.clone()));
        builder.guard.insert(0, Box::new(self.clone()));
        builder.dispatch.insert(0, Box::new(self));
    }
}
impl EsmtpService for DefaultMailService {
    fn prepare_session(&self, session: &mut SessionInfo) {
        if session.service_name.is_empty() {
            session.service_name = self.name.clone();
        }
    }
}

impl MailGuard for DefaultMailService {
    fn add_recipient<'a, 'f>(
        &'a self,
        request: AddRecipientRequest,
    ) -> S2Fut<'f, AddRecipientResult>
    where
        'a: 'f,
    {
        let AddRecipientRequest {
            mut transaction,
            rcpt,
        } = request;
        transaction.rcpts.push(rcpt);
        Box::pin(ready(AddRecipientResult::Accepted(transaction)))
    }
    fn start_mail<'a, 's, 'f>(
        &'a self,
        _session: &'s SessionInfo,
        mut request: StartMailRequest,
    ) -> S2Fut<'f, StartMailResult>
    where
        'a: 'f,
        's: 'f,
    {
        if request.id.is_empty() {
            let t = format!("{:?}", std::time::Instant::now());
            let id = md5::compute(&t);
            request.id = format!("{:x}", id);
        }
        Box::pin(ready(StartMailResult::Accepted(request)))
    }
}

impl MailDispatch for DefaultMailService {
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
        transaction.sink = Some(Box::pin(MailSink { id: id.clone() }));
        Box::pin(ready(Ok(transaction)))
    }
}

#[derive(Debug)]
pub struct MailSink {
    id: String,
}

impl Write for MailSink {
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        info!("Mail complete: {}", self.id,);
        Poll::Ready(Ok(()))
    }
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        debug!(
            "Mail data for {}: {:?}",
            self.id,
            String::from_utf8_lossy(buf)
        );
        Poll::Ready(Ok(buf.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_await_test::async_test;

    #[async_test]
    async fn test_id() {
        let sess = SessionInfo::default();
        let tran = Transaction::default();
        let sut = DefaultMailService::default();
        let tran = sut.start_mail(&sess, tran).await;
        if let StartMailResult::Accepted(tran) = tran {
            assert_ne!(tran.id, "");
        }
    }
}
