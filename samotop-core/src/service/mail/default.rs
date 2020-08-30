//! Reference implementation of a mail service
//! simply delivering mail to server console log.
use crate::common::*;
use crate::model::io::Connection;
use crate::model::mail::*;
use crate::service::mail::*;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct DefaultMailService;

impl NamedService for DefaultMailService {
    fn name(&self) -> &str {
        "samotop"
    }
}

impl EsmtpService for DefaultMailService {
    fn extend(&self, _connection: &mut Connection) {}
}

impl MailGuard for DefaultMailService {
    type RecipientFuture = futures::future::Ready<AcceptRecipientResult>;
    type SenderFuture = futures::future::Ready<AcceptSenderResult>;
    fn accept_recipient(&self, request: AcceptRecipientRequest) -> Self::RecipientFuture {
        future::ready(AcceptRecipientResult::Accepted(request.rcpt))
    }
    fn accept_sender(&self, _request: AcceptSenderRequest) -> Self::SenderFuture {
        future::ready(AcceptSenderResult::Accepted)
    }
}

impl MailQueue for DefaultMailService {
    type Mail = MailSink;
    type MailFuture = futures::future::Ready<Option<Self::Mail>>;

    fn mail(&self, envelope: Envelope) -> Self::MailFuture {
        let Envelope {
            ref name,
            ref peer,
            ref local,
            ref helo,
            ref mail,
            ref id,
            ref rcpts,
        } = envelope;
        println!(
            "Mail from {} (helo: {}, mailid: {}) (peer: {}) for {} on {} ({})",
            mail.as_ref()
                .map(|m| m.from().to_string())
                .unwrap_or("None".to_owned()),
            helo.as_ref()
                .map(|h| h.name().to_string())
                .unwrap_or("None".to_owned()),
            id,
            peer.as_ref()
                .map(|m| m.to_string())
                .unwrap_or("None".to_owned()),
            rcpts
                .iter()
                .fold(String::new(), |s, r| s + format!("{}, ", r).as_ref()),
            name,
            local
                .as_ref()
                .map(|m| m.to_string())
                .unwrap_or("None".to_owned())
        );
        future::ready(Some(MailSink { id: id.clone() }))
    }
    fn new_id(&self) -> String {
        Uuid::new_v4().to_string()
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
