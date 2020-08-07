use crate::model::io::*;
use crate::model::mail::*;
use crate::model::session::*;
use crate::model::{Error, Result};
use crate::service::mail::*;
use crate::service::session::*;
use bytes::Bytes;
use futures::prelude::*;
use pin_project::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};

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
    S: Send,
    M: Send,
    MFut: Send,
    GFut: Send,
    S: Clone,
    S: NamedService,
    S: MailGuard<Future = GFut>,
    S: MailQueue<MailFuture = MFut, Mail = M>,
    MFut: Future<Output = Option<M>>,
    GFut: Future<Output = AcceptRecipientResult>,
    M: Mail,
    M: Sink<Bytes, Error = Error>,
{
    type Handler = StatefulSessionHandler<S, M, MFut, GFut>;
    fn start(&self) -> Self::Handler {
        let name = self.mail_service.name();
        StatefulSessionHandler::new(name, self.mail_service.clone())
    }
}

enum HandlerState<M, MFut, GFut> {
    Ready,
    MailRcptChecking(Pin<Box<GFut>>),
    MailOpening(Pin<Box<MFut>>),
    MailDataWriting(Pin<Box<M>>),
    MailQueuing(Pin<Box<M>>),
    Closed,
}

#[pin_project(project=HandlerProjection)]
#[must_use = "streamsand sinks do nothing unless polled"]
pub struct StatefulSessionHandler<S, M, MFut, GFut> {
    mail_service: S,
    session: Session,
    state: HandlerState<M, MFut, GFut>,
}

impl<S, M, MFut, GFut> StatefulSessionHandler<S, M, MFut, GFut> {
    pub fn new(name: impl ToString, mail_service: S) -> Self {
        Self {
            mail_service,
            session: Session::new(name),
            state: HandlerState::Ready,
        }
    }
}

impl<S, M, MFut, GFut> Sink<ReadControl> for StatefulSessionHandler<S, M, MFut, GFut>
where
    S: MailGuard<Future = GFut>,
    S: MailQueue<MailFuture = MFut, Mail = M>,
    MFut: Future<Output = Option<M>>,
    GFut: Future<Output = AcceptRecipientResult>,
    M: Mail,
    M: Sink<Bytes, Error = Error>,
{
    type Error = Error;
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.poll_ready(cx)
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.poll_flush(cx)
    }
    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        let pending = || Poll::Pending;
        let ok = || Poll::Ready(Ok(()));
        let HandlerProjection { session, state, .. } = self.project();

        let mut poll = match state {
            HandlerState::Closed => ok(),
            HandlerState::Ready => ok(),
            HandlerState::MailRcptChecking(mail_guard_fut) => {
                // poll pending rcpt check
                match mail_guard_fut.as_mut().poll(cx) {
                    Poll::Pending => {
                        trace!("rcpt check not ready.");
                        pending()
                    }
                    Poll::Ready(result) => {
                        trace!("rcpt check ready!");
                        session.rcpt_checked(result);
                        *state = HandlerState::Ready;
                        ok()
                    }
                }
            }
            HandlerState::MailOpening(mail_fut) => {
                // poll pending mail send
                match mail_fut.as_mut().poll(cx) {
                    Poll::Pending => {
                        trace!("mail_fut not ready.");
                        pending()
                    }
                    Poll::Ready(None) => {
                        trace!("mail_fut ready and rejected!");
                        session.mail_sending(MailSendResult::Rejected);
                        *state = HandlerState::Ready;
                        ok()
                    }
                    Poll::Ready(Some(mail)) => {
                        trace!("mail_fut ready and accepted!");
                        session.mail_sending(MailSendResult::Ok);
                        *state = HandlerState::MailDataWriting(Box::pin(mail));
                        ok()
                    }
                }
            }
            HandlerState::MailDataWriting(mail) => {
                // poll the mail data sink
                match mail.as_mut().poll_flush(cx) {
                    Poll::Pending => {
                        trace!("mail sink not ready.");
                        pending()
                    }
                    Poll::Ready(Err(e)) => {
                        warn!("Sending mail data failed. {:?}", e);
                        session.error_sending_data();
                        *state = HandlerState::Ready;
                        ok()
                    }
                    Poll::Ready(Ok(())) => {
                        trace!("mail sink ready!");
                        ok()
                    }
                }
            }
            HandlerState::MailQueuing(mail) => match mail.as_mut().poll_close(cx) {
                Poll::Ready(Ok(())) => {
                    let id = mail.queue_id();
                    info!("Mail queued with ID {}", id);
                    session.mail_queued(QueueResult::QueuedWithId(id.to_string()));
                    *state = HandlerState::Ready;
                    ok()
                }
                Poll::Ready(Err(e)) => {
                    warn!("Mail queue with ID {} failed", mail.queue_id());
                    *state = HandlerState::Closed;
                    return Poll::Ready(Err(e));
                }
                Poll::Pending => {
                    trace!("mail sink close pending!");
                    pending()
                }
            },
        };

        if let Poll::Ready(Ok(())) = poll {
            if !session.is_ready() {
                trace!("session is not ready yet.");
                poll = pending();
            }
        }
        poll
    }

    fn start_send(self: Pin<&mut Self>, item: ReadControl) -> Result<()> {
        let dbg = format!("control sink item: {:?}", item);
        match self.project().session.control(item) {
            Ok(()) => {
                trace!("{}", dbg);
                Ok(())
            }
            Err(ControlResult::Wait) => {
                warn!("session was not ready when {} came", dbg);
                Err(format!("The session is not ready!").into())
            }
            Err(ControlResult::Ended) => {
                warn!("session already ended when {} came", dbg);
                Err(format!("The session is over!").into())
            }
        }
    }
}
impl<S, M, MFut, GFut> Stream for StatefulSessionHandler<S, M, MFut, GFut>
where
    S: MailGuard<Future = GFut>,
    S: MailQueue<MailFuture = MFut, Mail = M>,
    MFut: Future<Output = Option<M>>,
    GFut: Future<Output = AcceptRecipientResult>,
    M: Mail,
    M: Sink<Bytes, Error = Error>,
{
    type Item = Result<WriteControl>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        trace!("Polling next session answer.");
        // Pick an answer only in one place while passing the poll to the sink.
        // Whenever we later change the state we want to come back here right away
        // so we return a WriteControl::NoOp and the consumer will come back for more.
        let answer = match self.session.get_answer() {
            None => match self.session.is_closed() {
                true => {
                    trace!("No answer, the session is closed.");
                    return Poll::Ready(None);
                }
                false => {
                    trace!("No answer, pending.");
                    return Poll::Pending;
                }
            },
            Some(answer) => answer,
        };

        trace!("session control answer: {:?}", answer);

        let ok = |v| Poll::Ready(Some(Ok(v)));

        // process the answer
        match answer {
            SessionControl::Reply(reply) => ok(WriteControl::Reply(reply)),
            SessionControl::CheckRcpt(request) => {
                match self.state {
                    HandlerState::Ready => {
                        self.state = HandlerState::MailRcptChecking(Box::pin(
                            self.mail_service.accept(request),
                        ));
                        // we did something, but want to be called again
                        ok(WriteControl::NoOp)
                    }
                    _ => {
                        // This should not happen. Something is wrong with synchronization.
                        warn!("Asked to check Rcpt in a wrong state. Bummer!");
                        // We will not be adding this RCPT.
                        self.session.rcpt_checked(AcceptRecipientResult::Failed);
                        ok(WriteControl::NoOp)
                    }
                }
            }
            SessionControl::SendMail(envelope) => {
                match self.state {
                    HandlerState::Ready => {
                        self.state =
                            HandlerState::MailOpening(Box::pin(self.mail_service.mail(envelope)));
                        // we did something, but want to be called again
                        ok(WriteControl::NoOp)
                    }
                    _ => {
                        warn!("Asked to send mail win a wrongstate. Bummer!");
                        // I'm going to be very strict here. This should not happen.
                        self.session.mail_sending(MailSendResult::Failed);
                        // we did something, but want to be called again
                        ok(WriteControl::NoOp)
                    }
                }
            }
            SessionControl::Data(data) => {
                match self.state {
                    HandlerState::MailDataWriting(ref mut h) => {
                        match h.as_mut().poll_ready(cx) {
                            Poll::Ready(Ok(())) => match h.as_mut().start_send(data) {
                                Ok(()) => {
                                    /*Yay! Good stuff... */
                                    ok(WriteControl::NoOp)
                                }
                                Err(e) => {
                                    warn!("Mail data write error. {:?}", e);
                                    self.session.error_sending_data();
                                    ok(WriteControl::NoOp)
                                }
                            },
                            Poll::Ready(Err(e)) => {
                                warn!("Mail data write error. {:?}", e);
                                self.session.error_sending_data();
                                ok(WriteControl::NoOp)
                            }
                            Poll::Pending => {
                                /*Push back from the sink!*/
                                self.session.push_back(SessionControl::Data(data));
                                ok(WriteControl::NoOp)
                            }
                        }
                    }
                    _ => {
                        warn!("Asked to write mail data in a wrong state. Bummer!");
                        self.session.error_sending_data();
                        // we did something, but want to be called again
                        ok(WriteControl::NoOp)
                    }
                }
            }
            SessionControl::QueueMail => {
                let state = std::mem::replace(&mut self.state, HandlerState::Closed);
                match state {
                    HandlerState::MailDataWriting(h) => {
                        self.state = HandlerState::MailQueuing(h);
                        // we did something, but want to be called again
                        ok(WriteControl::NoOp)
                    }
                    _ => {
                        warn!("Asked to queue mail in a wrong state. Bummer!");
                        self.session.mail_queued(QueueResult::Failed);
                        // we did something, but want to be called again
                        ok(WriteControl::NoOp)
                    }
                }
            }
            SessionControl::AcceptStartTls => ok(WriteControl::StartTls),
            SessionControl::AcceptMailData => ok(WriteControl::StartData),
            SessionControl::EndOfSession => ok(WriteControl::Shutdown),
            SessionControl::Fail => Poll::Ready(Some(Err(format!("Mail session failed").into()))),
        }
    }
}
