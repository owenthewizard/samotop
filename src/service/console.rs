use super::MailService;
use bytes::Bytes;
use futures::{Async, AsyncSink, Poll, StartSend};
use hostname::get_hostname;
use model::mail::Envelope;
use tokio::io;
use tokio::prelude::*;

pub struct ConsoleMail {
    name: Option<String>,
}

impl ConsoleMail {
    pub fn new(name: Option<String>) -> Self {
        Self { name }
    }
}

impl MailService for ConsoleMail {
    type MailDataWrite = MailSink;
    fn name(&mut self) -> &str {
        self.name.get_or_insert_with(|| match get_hostname() {
            None => "Samotop".into(),
            Some(hostname) => hostname,
        })
    }
    fn send(&mut self, envelope: Envelope) -> Option<Self::MailDataWrite> {
        match envelope {
            Envelope {
                ref name,
                peer: Some(ref peer),
                local: Some(ref local),
                helo: Some(ref helo),
                mail: Some(ref mail),
                ref rcpts,
            } if rcpts.len() != 0 =>
            {
                println!(
                    "Mail from {} (helo: {}) (peer: {}) for {} on {} ({} <- {})",
                    mail.from(),
                    helo.name(),
                    peer,
                    rcpts
                        .iter()
                        .fold(String::new(), |s, r| s + format!("{}, ", r).as_ref()),
                    name,
                    local,
                    peer
                );
                Some(MailSink)
            }
            envelope => {
                warn!("Incomplete envelope: {:?}", envelope);
                None
            }
        }
    }
}

pub struct MailSink;

impl Sink for MailSink {
    type SinkItem = Bytes;
    type SinkError = io::Error;
    fn start_send(&mut self, bytes: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        println!("Mail data: {:?}", bytes);
        Ok(AsyncSink::Ready)
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        Ok(Async::Ready(()))
    }
}
