use super::MailService;
use bytes::{Bytes, IntoBuf};
use futures::{Async, AsyncSink, Poll, StartSend};
use hostname::get_hostname;
use model::session::Session;
use tokio::io;
use tokio::prelude::*;

pub struct ConsoleMail {
    name: Option<String>,
}

impl ConsoleMail {
    pub fn new() -> Self {
        Self { name: None }
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
    fn send(&mut self, session: &Session) -> Option<Self::MailDataWrite> {
        if let (Some(mail), Some(helo), Some(peer)) =
            (session.mail(), session.helo(), session.peer())
        {
            println!(
                "Mail from {} (helo: {}) (peer: {}) for {}",
                mail.from(),
                helo.name(),
                peer,
                session
                    .rcpts()
                    .fold(String::new(), |s, r| s + format!("{}, ", r).as_ref())
            );
            Some(MailSink)
        } else {
            None
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
