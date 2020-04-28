use bytes::Bytes;
use futures::{Async, AsyncSink, Poll, StartSend};
use model::mail::*;
use service::*;
use tokio::io;
use tokio::prelude::future::FutureResult;
use tokio::prelude::*;

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
    type Future = FutureResult<AcceptRecipientResult, io::Error>;
    fn accept(&self, request: AcceptRecipientRequest) -> Self::Future {
        println!("Accepting recipient {:?}", request);
        future::ok(AcceptRecipientResult::Accepted(request.rcpt))
    }
}

impl MailQueue for ConsoleMail {
    type Mail = MailSink;
    type MailFuture = FutureResult<Option<Self::Mail>, io::Error>;

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
                future::ok(Some(MailSink { id: id.clone() }))
            }
            envelope => {
                warn!("Incomplete envelope: {:?}", envelope);
                future::ok(None)
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

impl Mail for MailSink {
    fn queue(self) -> QueueResult {
        println!("Mail data finished for {}", self.id);
        QueueResult::QueuedWithId(self.id)
    }
}
