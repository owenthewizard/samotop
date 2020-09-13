//! Reference implementation of a mail service
//! simply delivering mail to server console log.
use crate::common::*;
use crate::model::mail::*;
use crate::service::mail::*;
use uuid::Uuid;

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

impl EsmtpService for DefaultMailService {
    fn prepare_session(&self, session: &mut SessionInfo) {
        if session.service_name.is_empty() {
            session.service_name = self.name.clone();
        }
    }
}

impl MailGuard for DefaultMailService {
    type RecipientFuture = futures::future::Ready<AddRecipientResult>;
    type SenderFuture = futures::future::Ready<StartMailResult>;
    fn add_recipient(&self, request: AddRecipientRequest) -> Self::RecipientFuture {
        let AddRecipientRequest {
            mut transaction,
            rcpt,
        } = request;
        transaction.rcpts.push(rcpt);
        future::ready(AddRecipientResult::Accepted(transaction))
    }
    fn start_mail(&self, mut request: StartMailRequest) -> Self::SenderFuture {
        if request.id.is_empty() {
            request.id = Uuid::new_v4().to_string();
        }
        future::ready(StartMailResult::Accepted(request))
    }
}

impl MailDispatch for DefaultMailService {
    type Mail = MailSink;
    type MailFuture = futures::future::Ready<DispatchResult<Self::Mail>>;

    fn send_mail(&self, transaction: Transaction) -> Self::MailFuture {
        let Transaction {
            ref session,
            ref mail,
            ref id,
            ref rcpts,
        } = transaction;
        println!(
            "Mail from {:?} for {} (mailid: {:?}). {}",
            mail.as_ref()
                .map(|m| m.from().to_string())
                .unwrap_or("nobody".to_owned()),
            rcpts
                .iter()
                .fold(String::new(), |s, r| s + format!("{:?}, ", r.to_string())
                    .as_ref()),
            id,
            session
        );
        future::ready(Ok(MailSink { id: id.clone() }))
    }
}

pub struct MailSink {
    id: String,
}

impl Write for MailSink {
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
        println!("Mail data for {}: {:?}", self.id, buf);
        Poll::Ready(Ok(buf.len()))
    }
}
