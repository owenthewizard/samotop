use bytes::Bytes;
use futures::StartSend;
use model::command::SmtpCommand;
use model::controll::{ClientControll, ServerControll};
use model::mail::*;
use model::response::SmtpReply;
use model::session::Session;
use service::*;
use tokio::io;
use tokio::prelude::*;
use util::*;

pub trait IntoMail
where
    Self: Sized,
{
    fn mail<M, W>(self, service: M) -> Mail<Self, M, W>
    where
        M: MailService<MailDataWrite = W>,
        W: MailHandler + Sink,
    {
        Mail::new(self, service)
    }
}

impl<S> IntoMail for S
where
    S: Stream,
{
}

pub struct Mail<S, M, W>
where
    W: MailHandler + Sink,
{
    stream: S,
    mail_service: M,
    state: Session,
    write: EventualMail<W>,
}

impl<S, M, W> Mail<S, M, W>
where
    W: MailHandler + Sink,
{
    pub fn new(stream: S, mail_service: M) -> Self {
        Self {
            stream,
            mail_service,
            state: Session::new(),
            write: EventualMail::new(),
        }
    }

    fn take_answer(&mut self) -> Option<ClientControll>
    where
        S: Stream<Item = ServerControll>,
        M: MailService<MailDataWrite = W>,
        M::MailDataWrite: Sink<SinkError = S::Error>,
        W: Sink<SinkItem = Bytes, SinkError = S::Error>,
    {
        let ctrl = match self.state.answer() {
            None => return None,
            Some(ctrl) => ctrl,
        };
        trace!("Answer: {:?}", ctrl);
        match ctrl {
            a @ ClientControll::AcceptData => {
                let envelope = self.state.extract_envelope();
                let write = self.mail_service.mail(envelope);
                match write {
                    None => {
                        // service did not accept the mail envelop
                        self.write.set(write);
                        self.state.cancel();
                        Some(reply_mail_not_accepted())
                    }
                    Some(_) => {
                        // service accepted the mail envelop
                        self.write.set(write);
                        Some(a)
                    }
                }
            }
            a @ ClientControll::QueueMail => {
                self.write.queue();
                Some(a)
            }
            a => Some(a),
        }
    }
}

impl<S, M, W> Stream for Mail<S, M, W>
where
    S: Stream<Item = ServerControll, Error = io::Error>,
    M: MailService<MailDataWrite = W>,
    M::MailDataWrite: Sink<SinkError = S::Error>,
    M::MailDataWrite: MailHandler,
    W: Sink<SinkItem = Bytes, SinkError = S::Error>,
{
    type Item = ClientControll;
    type Error = S::Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        // TODO: Do it only once
        // set service name
        self.state.set_name(self.mail_service.name());

        // pass any pending answers
        trace!("Handling any remaining answers");
        if let Some(ctrl) = self.take_answer() {
            return ok(ctrl);
        };

        // flush the mail data write sink if necessary
        trace!("Flushing the mail data write sink");
        try_ready!(self.write.poll_complete());

        trace!("Checking mail queue");
        match try_ready!(self.write.poll()) {
            None => { /*no buzz*/ }
            Some(QueueResult::Failed) => {
                self.state.cancel();
                return ok(reply_mail_not_accepted());
            }
            Some(QueueResult::Refused) => {
                self.state.cancel();
                return ok(reply_mail_queue_failed());
            }
            Some(QueueResult::QueuedWithId(_)) => {
                // should send back the id, but Ok is already given by session.
                // TODO: improve session handling in these edge cases
            }
        }

        // and handle the next stream item
        trace!("Fetching the next stream item");
        let ctrl = match try_ready!(self.stream.poll()) {
            None => {
                return none();
            }
            Some(c) => c,
        };

        let write = &mut self.write;

        trace!("Got controll: {:?}", ctrl);
        match ctrl {
            ServerControll::Command(SmtpCommand::Rcpt(ref rcpt)) => {
                match self.mail_service.accept(self.state.extract_rcpt(rcpt)) {
                    AcceptRecipientResult::Accepted(_) => {}
                    AcceptRecipientResult::AcceptedWithNewPath(_) => {}
                    AcceptRecipientResult::Rejected => return ok(reply_recipient_not_accepted()),
                    AcceptRecipientResult::RejectedWithNewPath(_) => {
                        return ok(reply_recipient_not_accepted())
                    }
                }
            }
            ServerControll::DataChunk(ref b) => {
                trace!("About to write...");
                match write.send(b.clone()).poll() {
                    Ok(Async::Ready(_)) => {
                        trace!("mail data chunk sent");
                    }
                    Ok(Async::NotReady) => {
                        trace!("mail data chunk not sent yet: {:?}", b);
                    }
                    Err(e) => {
                        warn!("mail data chunk send error");
                        return Err(e);
                    }
                }
            }
            _ => {}
        };

        trace!("Advancing machine state");
        self.state.controll(ctrl);

        trace!("Pending work will be done in next loop...");
        ok(ClientControll::Noop)
    }
}

fn reply_recipient_not_accepted() -> ClientControll {
    ClientControll::Reply(SmtpReply::MailboxNotAvailableFailure)
}
fn reply_mail_not_accepted() -> ClientControll {
    ClientControll::Reply(SmtpReply::MailboxNotAvailableFailure)
}
fn reply_mail_queue_failed() -> ClientControll {
    ClientControll::Reply(SmtpReply::MailboxNotAvailableError)
}

struct EventualMail<W> {
    sink: Option<W>,
    queued: Option<QueueResult>,
}

impl<W> EventualMail<W> {
    pub fn new() -> Self {
        Self {
            sink: None,
            queued: None,
        }
    }
    pub fn set(&mut self, sink: Option<W>) {
        self.sink = sink;
        self.queued = None;
    }
    pub fn queue(&mut self)
    where
        W: MailHandler,
    {
        self.queued = match self.sink.take() {
            None => panic!("trying to queue mail without a mail sink"),
            Some(w) => Some(w.queue()),
        };
    }
}

impl<W> Future for EventualMail<W> {
    type Item = Option<QueueResult>;
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.queued.take() {
            None => Ok(Async::Ready(None)),
            q @ Some(_) => Ok(Async::Ready(q)),
        }
    }
}

impl<W> Sink for EventualMail<W>
where
    W: Sink<SinkError = io::Error>,
{
    type SinkItem = W::SinkItem;
    type SinkError = W::SinkError;
    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        match self.sink {
            None => Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "Sink is not ready yet!",
            )),
            Some(ref mut w) => w.start_send(item),
        }
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        match self.sink {
            None => Ok(Async::Ready(())),
            Some(ref mut w) => w.poll_complete(),
        }
    }
}
