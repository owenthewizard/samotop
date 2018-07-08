use bytes::Bytes;
use futures::{Async, AsyncSink, Poll, StartSend};
use hostname::get_hostname;
use model::mail::*;
use service::*;
use tokio::io;
use tokio::prelude::*;

#[derive(Clone)]
pub struct ConsoleMail {
    name: Option<String>,
}

impl ConsoleMail {
    pub fn new(name: impl ToString) -> Self {
        Self {
            name: Some(name.to_string()),
        }
    }
    pub fn default() -> Self {
        Self { name: None }
    }
}

impl MailService for ConsoleMail {
    type MailDataWrite = MailSink;
    fn name(&self) -> String {
        match self.name {
            None => match get_hostname() {
                None => "Samotop".into(),
                Some(name) => name,
            },
            Some(ref name) => name.clone(),
        }
    }
    fn accept(&self, rcpt: AcceptRecipientRequest) -> AcceptRecipientResult {
        println!("Accepting recipient {:?}", rcpt);
        AcceptRecipientResult::Accepted
    }
    fn mail(&self, envelope: Envelope) -> Option<Self::MailDataWrite> {
        match envelope {
            Envelope {
                ref name,
                peer: Some(ref peer),
                local: Some(ref local),
                helo: Some(ref helo),
                mail: Some(ref mail),
                ref id,
                ref rcpts,
            } if rcpts.len() != 0 =>
            {
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
                Some(MailSink { id: id.clone() })
            }
            envelope => {
                warn!("Incomplete envelope: {:?}", envelope);
                None
            }
        }
    }
}

pub struct MailSink {
    id: String,
}

impl Sink for MailSink {
    type SinkItem = Bytes;
    type SinkError = io::Error;
    fn start_send(&mut self, bytes: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        println!("Mail data for {}: {:?}", self.id, bytes);
        Ok(AsyncSink::Ready)
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        Ok(Async::Ready(()))
    }
}

impl MailHandler for MailSink {
    fn into_queue(self) -> QueueResult {
        println!("Mail data finished for {}", self.id);
        QueueResult::QueuedWithId(self.id)
    }
}
