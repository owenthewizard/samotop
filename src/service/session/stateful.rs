use bytes::Bytes;
use futures::prelude::*;
use crate::model::controll::*;
use crate::model::mail::*;
use crate::model::session::*;
use crate::service::*;
use tokio::io;
use crate::util::*;

#[derive(Clone)]
pub struct StatefulSessionService<S> {
    mail_service: S,
}

impl<S> StatefulSessionService<S> {
    pub fn new(mail_service: S) -> Self {
        Self { mail_service }
    }
}

impl<S, M, MFut, GFut> SessionService for StatefulSessionService<S>
where
    S: Clone,
    S: NamedService,
    S: MailGuard<Future = GFut>,
    S: MailQueue<MailFuture = MFut, Mail = M>,
    MFut: Future<Item = Option<M>>,
    GFut: Future<Item = AcceptRecipientResult>,
    M: Mail,
    M: Sink<SinkItem = Bytes, SinkError = io::Error>,
{
    type Handler = StatefulSessionHandler<S, M, MFut, GFut>;
    fn start(&self, tls_conf: TlsControll) -> Self::Handler {
        let name = self.mail_service.name();
        StatefulSessionHandler::new(name, self.mail_service.clone(), tls_conf)
    }
}

pub struct StatefulSessionHandler<S, M, MFut, GFut> {
    mail_service: S,
    mail: Option<M>,
    mail_fut: Option<MFut>,
    mail_guard_fut: Option<GFut>,
    session: Session,
    tls_conf: TlsControll,
}

impl<S, M, MFut, GFut> StatefulSessionHandler<S, M, MFut, GFut> {
    pub fn new(name: impl ToString, mail_service: S, tls_conf: TlsControll) -> Self {
        Self {
            mail_service,
            mail: None,
            mail_fut: None,
            mail_guard_fut: None,
            session: Session::new(name),
            tls_conf,
        }
    }
}

impl<S, M, MFut, GFut> Sink for StatefulSessionHandler<S, M, MFut, GFut>
where
    S: MailGuard<Future = GFut>,
    S: MailQueue<MailFuture = MFut, Mail = M>,
    MFut: Future<Item = Option<M>, Error = io::Error>,
    GFut: Future<Item = AcceptRecipientResult, Error = io::Error>,
    M: Mail,
    M: Sink<SinkItem = Bytes, SinkError = io::Error>,
{
    type SinkItem = ServerControll;
    type SinkError = io::Error;
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        let mut poll = Ok(Async::Ready(()));
        let pending = || Ok(Async::NotReady);

        // poll pending mail send
        poll = match self.mail_fut.take() {
            None => poll,
            Some(mut f) => match f.poll() {
                Ok(Async::NotReady) => {
                    self.mail_fut = Some(f);
                    trace!("mail_fut not ready.");
                    pending()
                }
                Err(e) => {
                    warn!("Sending mail failed {:?}", e);
                    self.session.mail_sending(MailSendResult::Failed);
                    poll
                }
                Ok(Async::Ready(Some(mail))) => {
                    if self.mail.is_some() {
                        // This should not happen. Something is wrong with synchronization.
                        warn!("Got a new mail while another is in progress. Bummer!");
                        // We will not be going ahead with this new mail.
                        self.session.mail_sending(MailSendResult::Failed);
                        poll
                    } else {
                        trace!("mail_fut ready and accepted!");
                        self.mail = Some(mail);
                        self.session.mail_sending(MailSendResult::Ok);
                        poll
                    }
                }
                Ok(Async::Ready(None)) => {
                    trace!("mail_fut ready and rejected!");
                    self.session.mail_sending(MailSendResult::Rejected);
                    poll
                }
            },
        };

        // poll the mail data sink
        poll = match self.mail.take() {
            None => poll,
            Some(mut h) => match h.poll_complete() {
                Ok(Async::NotReady) => {
                    trace!("mail sink not ready.");
                    self.mail = Some(h);
                    pending()
                }
                Err(e) => {
                    warn!("Sending mail data failed. {:?}", e);
                    self.session.error_sending_data();
                    poll
                }
                Ok(Async::Ready(())) => {
                    trace!("mail sink ready!");
                    self.mail = Some(h);
                    poll
                }
            },
        };

        // poll pending rcpt check
        poll = match self.mail_guard_fut.take() {
            None => poll,
            Some(mut f) => match f.poll() {
                Err(e) => {
                    warn!("Checking mail recipient failed. {:?}", e);
                    self.session.rcpt_checked(AcceptRecipientResult::Failed);
                    poll
                }
                Ok(Async::NotReady) => {
                    trace!("rcpt check not ready.");
                    self.mail_guard_fut = Some(f);
                    pending()
                }
                Ok(Async::Ready(result)) => {
                    trace!("rcpt check ready!");
                    self.session.rcpt_checked(result);
                    poll
                }
            },
        };

        poll
    }
    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        let dbg = format!("controll sink item: {:?}", item);
        match self.session.controll(item) {
            ControllResult::Wait(item) => Ok(AsyncSink::NotReady(item)),
            ControllResult::Ok => {
                trace!("{}", dbg);
                Ok(AsyncSink::Ready)
            }
            ControllResult::Ended => {
                warn!("session already ended when {} came", dbg);
                Err(io::Error::new(
                    io::ErrorKind::NotConnected,
                    format!("The session is over!"),
                ))
            }
        }
    }
}
impl<S, M, MFut, GFut> Stream for StatefulSessionHandler<S, M, MFut, GFut>
where
    S: MailGuard<Future = GFut>,
    S: MailQueue<MailFuture = MFut, Mail = M>,
    MFut: Future<Item = Option<M>, Error = io::Error>,
    GFut: Future<Item = AcceptRecipientResult, Error = io::Error>,
    M: Mail,
    M: Sink<SinkItem = Bytes, SinkError = io::Error>,
{
    type Item = ClientControll;
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        // Pick an answer only in one place while passing the poll to the sink.
        // Whenever we later change the state we want to come back here right away
        // so we return a ClientControll::Noop and the consumer will come back for more.
        let answer = match self.session.get_answer() {
            None => match self.poll_complete() {
                Ok(Async::Ready(())) => match self.session.get_answer() {
                    None => return none(),
                    Some(answer) => answer,
                },
                Ok(Async::NotReady) => {
                    trace!("waiting for an answer.");
                    return pending();
                }
                Err(e) => return Err(e),
            },
            Some(answer) => answer,
        };

        trace!("session controll answer: {:?}", answer);

        // process the answer
        match answer {
            SessionControll::Reply(reply) => ok(ClientControll::Reply(reply)),
            SessionControll::CheckRcpt(request) => {
                if self.mail_guard_fut.is_some() {
                    // This should not happen. Something is wrong with synchronization.
                    warn!("Asked to check Rcpt while another check is in progress. Bummer!");
                    // We will not be adding this RCPT.
                    self.session.rcpt_checked(AcceptRecipientResult::Failed);
                    ok(ClientControll::Noop)
                } else {
                    self.mail_guard_fut = Some(self.mail_service.accept(request));
                    // we did something, but want to be called again
                    ok(ClientControll::Noop)
                }
            }
            SessionControll::SendMail(envelope) => {
                if self.mail.is_some() {
                    warn!("Asked to send mail while another one is in progress. Bummer!");
                    // I'm going to be very strict here. This should not happen.
                    self.session.mail_sending(MailSendResult::Failed);
                    // we did something, but want to be called again
                    ok(ClientControll::Noop)
                } else {
                    self.mail_fut = Some(self.mail_service.mail(envelope));
                    // we did something, but want to be called again
                    ok(ClientControll::Noop)
                }
            }
            SessionControll::Data(data) => {
                match self.mail {
                    None => {
                        warn!("Asked to write mail data without a handler. Bummer!");
                        self.session.error_sending_data();
                        // we did something, but want to be called again
                        ok(ClientControll::Noop)
                    }
                    Some(ref mut h) => {
                        match h.start_send(data) {
                            Ok(AsyncSink::Ready) => {
                                /*Yay! Good stuff... */
                                ok(ClientControll::Noop)
                            }
                            Ok(AsyncSink::NotReady(data)) => {
                                /*Push back from the sink!*/
                                self.session.push_back(SessionControll::Data(data));
                                ok(ClientControll::Noop)
                            }
                            Err(e) => {
                                warn!("Mail data write error. {:?}", e);
                                self.session.error_sending_data();
                                ok(ClientControll::Noop)
                            }
                        }
                    }
                }
            }
            SessionControll::QueueMail => {
                match self.mail.take() {
                    None => {
                        warn!("Asked to queue mail without a handler. Bummer!");
                        self.session.mail_queued(QueueResult::Failed);
                        // we did something, but want to be called again
                        ok(ClientControll::Noop)
                    }
                    Some(h) => {
                        let result = h.queue();
                        self.session.mail_queued(result);
                        // we did something, but want to be called again
                        ok(ClientControll::Noop)
                    }
                }
            }
            SessionControll::AcceptStartTls => {
                self.tls_conf.start_tls();
                ok(ClientControll::Noop)
            }
            SessionControll::AcceptMailData(accept) => ok(ClientControll::AcceptData(accept)),
            SessionControll::EndOfSession => ok(ClientControll::Shutdown),
            SessionControll::Fail => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Mail session failed"),
            )),
        }
    }
}
