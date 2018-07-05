use bytes::{Buf, Bytes, BytesMut, IntoBuf};
use futures::sink;
use futures::StartSend;
use model::controll::{ClientControll, ServerControll};
use model::response::SmtpReply;
use model::session::Session;
use service::MailService;
use std::mem;
use tokio::io;
use tokio::prelude::*;
use util::futu::*;

pub trait IntoMail
where
    Self: Sized,
{
    fn mail<M, W>(self, service: M) -> Mail<Self, M, W>
    where
        M: MailService<MailDataWrite = W>,
        W: Sink,
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
    W: Sink,
{
    stream: S,
    service: M,
    state: Session,
    write: EventualSink<W>,
}

impl<S, M, W> Mail<S, M, W>
where
    W: Sink,
{
    pub fn new(stream: S, service: M) -> Self {
        Self {
            stream,
            service,
            state: Session::new(),
            write: EventualSink { sink: None },
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
            None => {
                return None;
            }
            Some(ctrl) => ctrl,
        };
        trace!("Answer: {:?}", ctrl);
        match ctrl {
            a @ ClientControll::AcceptData => {
                match self.write.set(self.service.send(&self.state)) {
                    Err(_) => Some(reply_mail_not_accepted()),
                    Ok(()) => Some(a),
                }
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
    W: Sink<SinkItem = Bytes, SinkError = S::Error>,
{
    type Item = ClientControll;
    type Error = S::Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        // TODO: Do it only once
        // set service name
        self.state.set_name(self.service.name());

        // pass any pending answers
        trace!("Handling any remaining answers");
        if let Some(ctrl) = self.take_answer() {
            return ok(ctrl);
        };

        // flush the mail data write sink if necessary
        trace!("Flushing the mail data write sink");
        try_ready!(self.write.poll_complete());

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
            ServerControll::DataChunk(ref b) => {
                // TODO: keep the buffer and poll again
                //let mut buf = Bytes::from(&b[..]).into_buf();
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
            /*
            ServerControll::FinalDot(_) => if let Some(ref mut w) = self.write {
                // TODO: poll again if not done
                trace!("About to flush...");
                error!("unimplemented!");
            } else {
                warn!("Got data but no writer!");
            },*/
            _ => {}
        };

        trace!("Advancing machine state");
        self.state.controll(ctrl);

        trace!("Pending work will be done in next loop...");
        ok(ClientControll::Noop)
    }
}

fn reply_mail_not_accepted() -> ClientControll {
    ClientControll::Reply(SmtpReply::MailboxNotAvailableFailure)
}

struct EventualSink<W> {
    sink: Option<W>,
}

impl<W> EventualSink<W> {
    pub fn new() -> Self {
        Self { sink: None }
    }
    pub fn set(&mut self, sink: Option<W>) -> io::Result<()> {
        self.sink = sink;
        Ok(())
    }
}

impl<W> Sink for EventualSink<W>
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
