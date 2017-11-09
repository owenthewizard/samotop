use std::io;
use bytes::Bytes;
use futures::sync::mpsc::Sender;
use futures::{Stream, Sink, Async, AsyncSink, StartSend, Poll};
use tokio_proto::streaming::pipeline::Transport;
use model::request::*;
use model::response::SmtpReply;
use model::act::*;

pub struct ActTransport<TT> {
    upstream: TT,
    pending: Option<SmtpReply>,
    conn: Option<SmtpConnection>,
    helo: Option<SmtpHelo>,
    mail: Option<SmtpMail>,
    rcpt: Option<Vec<SmtpPath>>,
    data: Option<Sender<Bytes>>,
}

impl<TT> ActTransport<TT>
where
    TT: Sink<SinkItem = SmtpReply, SinkError = io::Error>,
    TT: 'static,
{
    pub fn new(upstream: TT) -> Self {
        Self {
            upstream,
            pending: None,
            conn: None,
            helo: None,
            mail: None,
            rcpt: None,
            data: None,
        }
    }
    fn state(&self) -> State {
        match self.conn {
            None => State::New,
            Some(_) => {
                match self.helo {
                    None => State::Connected,
                    Some(_) => {
                        match self.mail {
                            None => State::Helo,
                            Some(_) => {
                                match self.rcpt {
                                    None => State::MailFrom,
                                    Some(_) => {
                                        match self.data {
                                            None => State::RcptTo,
                                            Some(_) => State::Data,
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    fn reply(&mut self, reply: SmtpReply) -> StartSend<SmtpReply, io::Error> {
        self.upstream.start_send(reply)
    }
    fn reply_to_poll(&mut self, reply: SmtpReply) -> Poll<Option<Act>, io::Error> {
        // Intercept PING messages and send back a PONG
        let res = try!(self.reply(reply));

        // Ideally, the case of the sink not being ready
        // should be handled. See the link to the full
        // example below.
        assert!(res.is_ready());

        // Try flushing the pong, only bubble up errors
        try!(self.poll_complete());

        none()
    }
    fn reply_to_act(
        &mut self,
        result: ActResult,
        reply: SmtpReply,
    ) -> StartSend<ActResult, io::Error> {
        match try!(self.reply(reply)) {
            AsyncSink::Ready => Ok(AsyncSink::Ready),
            AsyncSink::NotReady(_) => Ok(AsyncSink::NotReady(result)),
        }
    }
}

#[derive(Debug)]
enum State {
    New,
    Connected,
    Helo,
    MailFrom,
    RcptTo,
    Data,
}

fn none() -> Poll<Option<Act>, io::Error> {
    Ok(Async::Ready(None))
}

fn notready() -> Poll<Option<Act>, io::Error> {
    Ok(Async::NotReady)
}

fn some(act: Act) -> Poll<Option<Act>, io::Error> {
    Ok(Async::Ready(Some(act)))
}

impl<TT> Stream for ActTransport<TT>
where
    TT: 'static + Stream<Item = SmtpInput, Error = io::Error>,
    TT: Sink<SinkItem = SmtpReply, SinkError = io::Error>,
{
    type Error = io::Error;
    type Item = Act;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        trace!("poll...");
        loop {
            trace!("upstream poll...");
            // Poll the upstream transport. `try_ready!` will bubble up
            // errors and Async::NotReady.
            let poll = self.upstream.poll();
            trace!("upstream poll result: {:?}", poll);
            if let Some(inp) = try_ready!(poll) {
                let state = self.state();
                trace!("Got {:?} in state {:?}", inp, state);
                try_ready!(match state {
                    State::New => {
                        match inp {
                            SmtpInput::Connect(c) => {
                                self.conn = Some(c);
                                self.reply_to_poll(SmtpReply::ServiceReadyInfo("Oy".to_string()))
                            }
                            _ => self.reply_to_poll(SmtpReply::CommandSequenceFailure),
                        }
                    }
                    State::Connected => {
                        match inp {
                            SmtpInput::Command(_, _, SmtpCommand::Helo(h)) => {
                                self.helo = Some(h);
                                self.reply_to_poll(SmtpReply::OkInfo)
                            }
                            _ => self.reply_to_poll(SmtpReply::CommandSequenceFailure),
                        }
                    }
                    State::Helo => {
                        match inp {
                            SmtpInput::Command(_, _, SmtpCommand::Mail(m)) => {
                                self.mail = Some(m);
                                self.reply_to_poll(SmtpReply::OkInfo)
                            }
                            _ => self.reply_to_poll(SmtpReply::CommandSequenceFailure),
                        }
                    }
                    State::MailFrom => self.reply_to_poll(SmtpReply::CommandNotImplementedFailure),
                    State::RcptTo => self.reply_to_poll(SmtpReply::CommandNotImplementedFailure),
                    State::Data => self.reply_to_poll(SmtpReply::CommandNotImplementedFailure),
                });
            } else {
                trace!("Got None");
                return none();
            }
        }
    }
}

impl<TT> Sink for ActTransport<TT>
where
    TT: 'static + Sink<SinkError = io::Error, SinkItem = SmtpReply>,
{
    type SinkError = io::Error;
    type SinkItem = ActResult;

    fn start_send(&mut self, rsp: ActResult) -> StartSend<ActResult, io::Error> {
        match self.state() {
            State::Connected => {
                match rsp {
                    Err(_) => self.reply_to_act(rsp, SmtpReply::ProcesingError),
                    Ok(_) => self.reply_to_act(rsp, SmtpReply::OkInfo),
                }
            }
            _ => self.reply_to_act(rsp, SmtpReply::ProcesingError),
        }
    }

    fn poll_complete(&mut self) -> Poll<(), io::Error> {
        self.upstream.poll_complete()
    }

    fn close(&mut self) -> Poll<(), io::Error> {
        self.upstream.close()
    }
}

impl<TT> Transport for ActTransport<TT>
where
    TT: 'static,
    TT: Stream<Error = io::Error, Item = SmtpInput>,
    TT: Sink<SinkError = io::Error, SinkItem = SmtpReply>,
{
}
