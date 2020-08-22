//! Reference implementation of a mail service
//! simply delivering mail to server console log.
use crate::common::*;
use crate::model::io::Connection;
use crate::model::mail::*;
use crate::model::Error;
use crate::service::mail::*;

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
    type Future = futures::future::Ready<AcceptRecipientResult>;
    fn accept(&self, request: AcceptRecipientRequest) -> Self::Future {
        future::ready(AcceptRecipientResult::Accepted(request.rcpt))
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
}

pub struct MailSink {
    id: String,
}

impl Sink<Vec<u8>> for MailSink {
    type Error = Error;
    fn start_send(self: Pin<&mut Self>, bytes: Vec<u8>) -> Result<()> {
        println!("Mail data for {}: {:?}", self.id, bytes);
        Ok(())
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.poll_ready(cx)
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.poll_flush(cx)
    }
    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }
}
