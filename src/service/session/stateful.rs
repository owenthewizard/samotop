use bytes::Bytes;
use futures::prelude::*;
use model::controll::*;
use model::mail::*;
use model::session::*;
use service::*;
use tokio::io;
use util::*;

#[derive(Clone)]
pub struct StatefulSessionService<M> {
    mail_service: M,
}

impl<M> StatefulSessionService<M> {
    pub fn new(mail_service: M) -> Self {
        Self { mail_service }
    }
}

impl<M, H> SessionService for StatefulSessionService<M>
where
    M: MailService<MailDataWrite = H> + Clone,
{
    type Handler = StatefulSessionHandler<M, H>;
    fn start(&self) -> Self::Handler {
        let name = self.mail_service.name();
        StatefulSessionHandler::new(name, self.mail_service.clone())
    }
}

pub struct StatefulSessionHandler<M, H> {
    mail_service: M,
    mail_handler: Option<H>,
    session: Session,
}

impl<M, H> StatefulSessionHandler<M, H> {
    pub fn new(name: impl ToString, mail_service: M) -> Self {
        Self {
            mail_service,
            mail_handler: None,
            session: Session::new(name),
        }
    }
}

impl<M, H> Sink for StatefulSessionHandler<M, H>
where
    H: Sink<SinkError = io::Error>,
{
    type SinkItem = ServerControll;
    type SinkError = io::Error;
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        match self.mail_handler {
            None => {}
            Some(ref mut h) => match h.poll_complete() {
                nr @ Ok(Async::NotReady) => return nr,
                Err(e) => return Err(e),
                Ok(Async::Ready(())) => {}
            },
        };
        Ok(Async::Ready(()))
    }
    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        match self.session.controll(item) {
            ControllResult::Ok => Ok(AsyncSink::Ready),
            ControllResult::Wait(item) => Ok(AsyncSink::NotReady(item)),
            ControllResult::Ended => Err(io::Error::new(
                io::ErrorKind::NotConnected,
                format!("The session is over!"),
            )),
        }
    }
}
impl<M, H> Stream for StatefulSessionHandler<M, H>
where
    M: MailService<MailDataWrite = H>,
    H: MailHandler,
    H: Sink<SinkItem = Bytes, SinkError = io::Error>,
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
                Ok(Async::NotReady) => return pending(),
                Err(e) => return Err(e),
            },
            Some(answer) => answer,
        };

        match answer {
            SessionControll::Reply(reply) => ok(ClientControll::Reply(reply)),
            SessionControll::CheckRcpt(request) => {
                let result = self.mail_service.accept(request);
                self.session.rcpt_checked(result);
                // we did something, but want to be called again
                ok(ClientControll::Noop)
            }
            SessionControll::SendMail(envelope) => {
                if self.mail_handler.is_some() {
                    warn!("Asked to send mail, while another one is in progress. Bummer!");
                    // I'm going to be very strict here. This should not happen.
                    self.session.mail_sent(MailSendResult::Failed);
                    // we did something, but want to be called again
                    ok(ClientControll::Noop)
                } else {
                    let result = self.mail_service.mail(envelope);
                    match result {
                        None => {
                            self.session.mail_sent(MailSendResult::Rejected);
                            // we did something, but want to be called again
                            ok(ClientControll::Noop)
                        }
                        Some(h) => {
                            self.session.mail_sent(MailSendResult::Ok);
                            self.mail_handler = Some(h);
                            ok(ClientControll::AcceptData)
                        }
                    }
                }
            }
            SessionControll::Data(data) => {
                match self.mail_handler {
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
                match self.mail_handler.take() {
                    None => {
                        warn!("Asked to queue mail without a handler. Bummer!");
                        self.session.queued(QueueResult::Failed);
                        // we did something, but want to be called again
                        ok(ClientControll::Noop)
                    }
                    Some(h) => {
                        let result = h.queue();
                        self.session.queued(result);
                        // we did something, but want to be called again
                        ok(ClientControll::Noop)
                    }
                }
            }
            SessionControll::EndOfSession => ok(ClientControll::Shutdown),
        }
    }
}
