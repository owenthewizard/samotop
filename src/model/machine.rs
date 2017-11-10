use std::fmt;
use std::io::{Read, Write, Error};
use futures::{Stream, Sink, AsyncSink, Poll, Async, Future};
use tokio_proto::streaming::Body;
use model::request::*;
use model::response::{SmtpReply, SmtpExtension};
use super::state::*;
use super::act;

const EXTENSIONS: &'static [SmtpExtension] = &[];

// machine entry point
pub fn start<D>(dispatch: D) -> SmtpMachine<Session, D> {
    SmtpMachine {
        state: Session,
        replies: vec![],
        dispatch,
    }
}

pub struct SmtpMachine<S, D> {
    state: S,
    dispatch: D,
    replies: Vec<SmtpReply>,
}

// all the machines have this
impl<S, D> SmtpMachine<S, D> {
    pub fn state<'a>(&'a self) -> &'a S {
        &self.state
    }
    pub fn noop(self) -> Self {
        self.step(SmtpReply::OkInfo, |s| s)
    }
    pub fn quit(self) -> SmtpMachine<Closed, D> {
        self.step(
            SmtpReply::ClosingConnectionInfo(format!("Ciao!")),
            |_| Closed,
        )
    }
    pub fn help(self, _: Vec<String>) -> SmtpMachine<S, D> {
        self.step(SmtpReply::CommandNotImplementedFailure, |s| s)
    }
    pub fn expn(self, _: String) -> SmtpMachine<S, D> {
        self.step(SmtpReply::CommandNotImplementedFailure, |s| s)
    }
    pub fn vrfy(self, _: String) -> SmtpMachine<S, D> {
        self.step(SmtpReply::CommandNotImplementedFailure, |s| s)
    }
    pub fn unexpected(self, _: SmtpInput) -> SmtpMachine<S, D> {
        self.step(SmtpReply::CommandSequenceFailure, |s| s)
    }
    fn step<R, F: FnOnce(S) -> R>(self, reply: SmtpReply, next: F) -> SmtpMachine<R, D> {
        let SmtpMachine {
            state,
            mut replies,
            dispatch,
        } = self;
        replies.push(reply);
        SmtpMachine {
            state: next(state),
            replies: replies,
            dispatch: dispatch,
        }
    }
    fn helo_reply(&self, conn: &SmtpConnection, helo: &SmtpHelo) -> SmtpReply {
        use self::SmtpHelo::*;
        match helo {
            &Helo(_) => SmtpReply::OkHeloInfo {
                local: conn.local_name.to_owned(),
                remote: helo.name(),
            },
            &Ehlo(_) => SmtpReply::OkEhloInfo {
                local: conn.local_name.to_owned(),
                remote: helo.name(),
                extensions: EXTENSIONS.into(),
            },
        }
    }
}

impl<D> SmtpMachine<Session, D> {
    pub fn reset(self) -> SmtpMachine<Session, D> {
        self.step(SmtpReply::OkInfo, |s| s)
    }
    pub fn connect(self, conn: SmtpConnection) -> SmtpMachine<Conn, D> {
        self.step(
            SmtpReply::ServiceReadyInfo(conn.local_name.to_owned()),
            |_| Conn { conn },
        )
    }
}

impl<D> SmtpMachine<Conn, D> {
    pub fn reset(self) -> SmtpMachine<Conn, D> {
        self.step(SmtpReply::OkInfo, |s| s)
    }
    pub fn helo(self, helo: SmtpHelo) -> SmtpMachine<Helo, D> {
        let reply = self.helo_reply(&self.state.conn, &helo);
        self.step(reply, |s| Helo { conn: s.conn, helo })
    }
}

impl<D> SmtpMachine<Helo, D> {
    pub fn reset(self) -> SmtpMachine<Helo, D> {
        self.step(SmtpReply::OkInfo, |s| {
            Helo {
                conn: s.conn,
                helo: s.helo,
            }
        })
    }
    pub fn helo(self, helo: SmtpHelo) -> SmtpMachine<Helo, D> {
        self.step(SmtpReply::OkInfo, |s| Helo { conn: s.conn, helo })
    }
    pub fn mail(self, mail: SmtpMail) -> SmtpMachine<Mail, D> {
        self.step(SmtpReply::OkInfo, |s| {
            Mail {
                mail,
                helo: s.helo,
                conn: s.conn,
            }
        })
    }
}

impl<D> SmtpMachine<Mail, D> {
    pub fn reset(self) -> SmtpMachine<Helo, D> {
        self.step(SmtpReply::OkInfo, |s| {
            Helo {
                conn: s.conn,
                helo: s.helo,
            }
        })
    }
    pub fn helo(self, helo: SmtpHelo) -> SmtpMachine<Helo, D> {
        self.step(SmtpReply::OkInfo, |s| Helo { conn: s.conn, helo })
    }
    pub fn rcpt(self, path: SmtpPath) -> SmtpMachine<Rcpt, D> {
        self.step(SmtpReply::OkInfo, |s| {
            Rcpt {
                rcpt: vec![path],
                mail: s.mail,
                helo: s.helo,
                conn: s.conn,
            }
        })
    }
}

impl<D> SmtpMachine<Rcpt, D> {
    pub fn reset(self) -> SmtpMachine<Helo, D> {
        self.step(SmtpReply::OkInfo, |s| {
            Helo {
                conn: s.conn,
                helo: s.helo,
            }
        })
    }
    pub fn helo(self, helo: SmtpHelo) -> SmtpMachine<Helo, D> {
        self.step(SmtpReply::OkInfo, |s| Helo { conn: s.conn, helo })
    }
    pub fn rcpt(self, path: SmtpPath) -> SmtpMachine<Rcpt, D> {
        self.step(SmtpReply::OkInfo, |s| {
            let Rcpt {
                mut rcpt,
                mail,
                helo,
                conn,
            } = s;
            rcpt.push(path);
            Rcpt {
                rcpt,
                mail,
                helo,
                conn,
            }
        })
    }
    pub fn data(self, write: Box<Write>, read: Box<Read>) -> SmtpMachine<Data, D> {
        self.step(SmtpReply::StartMailInputChallenge, |s| {
            let (tx, body) = Body::pair();
            Data {
                rcpt: s.rcpt,
                mail: s.mail,
                helo: s.helo,
                conn: s.conn,
                body,
                tx,
            }
        })
    }
}
impl<D> SmtpMachine<Data, D> {
    pub fn done(self) -> SmtpMachine<act::Mail, D> {
        self.step(SmtpReply::OkInfo, |s| {
            act::new_mail(s.conn, s.helo, s.mail, s.rcpt)
        })
    }
}
impl<D> SmtpMachine<act::Mail, D> {
    pub fn ok(self) -> SmtpMachine<Helo, D> {
        self.step(SmtpReply::OkInfo, |s| {
            Helo {
                helo: s.helo,
                conn: s.conn,
            }
        })
    }
    pub fn failed(self) -> SmtpMachine<Helo, D> {
        self.step(SmtpReply::TransactionFailure, |s| {
            Helo {
                helo: s.helo,
                conn: s.conn,
            }
        })
    }
}

impl<S, D> Stream for SmtpMachine<S, D>
where
    D: Sink<SinkItem = SmtpReply, SinkError = Error>,
    D: Stream<Item = SmtpInput, Error = Error>,
{
    type Item = SmtpInput;
    type Error = Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        // first pull from stream
        match self.dispatch.poll() {
            Err(e) => Err(e),
            item @ Ok(Async::Ready(_)) => item,
            Ok(Async::NotReady) => {
                // if empty, send to sink
                if self.replies.is_empty() {
                    try_ready!(self.dispatch.poll_complete());
                    Ok(Async::NotReady)
                } else {
                    match self.dispatch.start_send(self.replies.remove(0)) {
                        Err(e) => Err(e),
                        Ok(AsyncSink::Ready) => Ok(Async::NotReady),
                        Ok(AsyncSink::NotReady(reply)) => {
                            self.replies.insert(0, reply);
                            Ok(Async::NotReady)
                        }
                    }
                }
            }
        }
    }
}

impl<S, D> fmt::Debug for SmtpMachine<S, D>
where
    S: fmt::Debug,
{
    fn fmt<'a>(&self, fmt: &mut fmt::Formatter<'a>) -> Result<(), fmt::Error> {
        fmt.write_fmt(format_args!(
            "StateMachine{{ state: {:?}, replies: {:?}}}",
            self.state,
            self.replies
        ))
    }
}

#[cfg(test)]
mod tests {
    use env_logger;
    use futures::StartSend;
    use super::*;

    #[derive(Debug)]
    struct DebugSink;
    impl Sink for DebugSink {
        type SinkItem = SmtpReply;
        type SinkError = Error;
        fn start_send(
            &mut self,
            item: Self::SinkItem,
        ) -> StartSend<Self::SinkItem, Self::SinkError> {
            trace!("Sending:\r\n{}", item);
            Ok(AsyncSink::Ready)
        }
        fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
            trace!("Poll complete");
            Ok(Async::Ready(()))
        }
    }
    impl Stream for DebugSink {
        type Item = SmtpInput;
        type Error = Error;
        fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
            Ok(Async::Ready(None))
        }
    }

    #[test]
    fn go2() {
        env_logger::init().unwrap();

        let machine = start(DebugSink);

        let machine = machine.connect(SmtpConnection {
            local_name: "test".to_string(),
            peer_addr: None,
            local_addr: None,
        });

        let mut machine = machine.helo(SmtpHelo::Ehlo(SmtpHost::Domain("go2".to_string())));

        trace!("{:?}", machine);

        machine.poll().unwrap();
        machine.poll().unwrap();
        machine.poll().unwrap();

        match machine.state() {
            &Helo { .. } => (),
            _ => (),
        }
    }
}
