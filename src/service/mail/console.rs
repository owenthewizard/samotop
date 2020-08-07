//! Reference implementation of a mail service 
//! simply delivering mail to server console log.
//! 
//! If you wish to implement your own mail service with Samotop,
//! copy this file (`ConsoleMail`) and customize it.
use crate::model::mail::*;
use crate::model::Error;
use crate::service::mail::*;
use bytes::Bytes;
use futures::prelude::*;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Clone)]
pub struct ConsoleMail {
    name: String,
}

impl ConsoleMail {
    pub fn new(name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl NamedService for ConsoleMail {
    fn name(&self) -> &str {
        self.name.as_str()
    }
}

impl MailGuard for ConsoleMail {
    type Future = futures::future::Ready<AcceptRecipientResult>;
    fn accept(&self, request: AcceptRecipientRequest) -> Self::Future {
        println!("Accepting recipient {:?}", request);
        future::ready(AcceptRecipientResult::Accepted(request.rcpt))
    }
}

impl MailQueue for ConsoleMail {
    type Mail = MailSink;
    type MailFuture = futures::future::Ready<Option<Self::Mail>>;

    fn mail(&self, envelope: Envelope) -> Self::MailFuture {
        match envelope {
            Envelope {
                ref name,
                peer: Some(ref peer),
                local: Some(ref local),
                helo: Some(ref helo),
                mail: Some(ref mail),
                ref id,
                ref rcpts,
            } if rcpts.len() != 0 => {
                println!(
                    "Mail from {} (helo: {}, mailid: {}) (peer: {}) for {} on {} ({} <- {})",
                    mail.from(),
                    helo.name(),
                    id,
                    peer,
                    rcpts
                        .iter()
                        .fold(String::new(), |s, r| s + format!("{}, ", r).as_ref()),
                    name,
                    local,
                    peer
                );
                future::ready(Some(MailSink { id: id.clone() }))
            }
            envelope => {
                warn!("Incomplete envelope: {:?}", envelope);
                future::ready(None)
            }
        }
    }
}

pub struct MailSink {
    id: String,
}

impl Sink<Bytes> for MailSink {
    type Error = Error;
    fn start_send(self: Pin<&mut Self>, bytes: Bytes) -> Result<(), Self::Error> {
        println!("Mail data for {}: {:?}", self.id, bytes);
        Ok(())
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_ready(cx)
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_flush(cx)
    }
    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

impl Mail for MailSink {
    fn queue_id(&self) -> &str {
        self.id.as_ref()
    }
}
