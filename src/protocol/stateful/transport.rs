use std::io;
use bytes::Bytes;
use futures::{Stream, Sink, Async, AsyncSink, StartSend, Poll};
use futures::sync::mpsc::Sender;
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
    TT: 'static + Stream<Error = io::Error, Item = SmtpInput>,
    TT: Sink<SinkItem = SmtpReply, SinkError = io::Error>,
{
    type Error = io::Error;
    type Item = Act;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
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







#[cfg(test)]
mod tests {
    use env_logger;
    use std::io;
    use std::error::Error;
    use std::fmt::Debug;
    use std::sync::mpsc::{Sender, Receiver, channel};
    use futures::{Sink, Stream, Async, AsyncSink, Poll, StartSend};
    use tokio_proto::streaming::pipeline::Transport;
    use model::request::*;
    use model::response::SmtpReply;
    use super::ActTransport;

    type Sut = ActTransport<MockTransport<SmtpInput, SmtpReply>>;

    #[test]
    fn new_creates() {

        let (upstream, _tx_inp, _rx_rpl) = MockTransport::setup();

        let _sut = Sut::new(upstream);
    }

    #[test]
    fn streams_mail() {

        env_logger::init().unwrap();

        let (upstream, tx_inp, _rx_rpl) = MockTransport::setup();

        let mut sut = Sut::new(upstream);

        tx_inp.send(Ok(Async::Ready(Some(SmtpInput::Connect(SmtpConnection {
            peer_addr: None,
            local_addr: None,
        })))));
        tx_inp.send(Ok(Async::Ready(None)));

        let result = sut.poll();

        match sut.poll() {
            Ok(Async::NotReady) => (),
            _ => panic!(""),
        }
    }



    pub struct MockTransport<TIn, TOut> {
        pub stream: Receiver<Result<Async<Option<TIn>>, io::Error>>,
        pub sink: Sender<TOut>,
    }
    impl<TIn, TOut> MockTransport<TIn, TOut> {
        pub fn setup() -> (Self, Sender<Result<Async<Option<TIn>>, io::Error>>, Receiver<TOut>) {

            let (tx_cmd, rx_cmd): (Sender<Result<Async<Option<TIn>>, io::Error>>,
                                   Receiver<Result<Async<Option<TIn>>, io::Error>>) = channel();
            let (tx_rpl, rx_rpl): (Sender<TOut>, Receiver<TOut>) = channel();
            (
                Self {
                    sink: tx_rpl,
                    stream: rx_cmd,
                },
                tx_cmd,
                rx_rpl,
            )
        }
    }
    impl<TIn, TOut> Stream for MockTransport<TIn, TOut>
    where
        TIn: Debug,
    {
        type Item = TIn;
        type Error = io::Error;
        fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
            let inp = self.stream.recv();
            trace!("{:?}", inp);
            match inp {
                Err(e) => Err(io::Error::new(io::ErrorKind::Other, e.description())),
                Ok(f) => f,
            }
        }
    }
    impl<TIn, TOut> Sink for MockTransport<TIn, TOut> {
        type SinkItem = TOut;
        type SinkError = io::Error;

        fn start_send(&mut self, request: Self::SinkItem) -> StartSend<Self::SinkItem, io::Error> {
            match self.sink.send(request) {
                Ok(_) => Ok(AsyncSink::Ready),
                Err(_) => Err(io::ErrorKind::Other.into()),
            }
        }

        fn poll_complete(&mut self) -> Poll<(), io::Error> {
            Ok(Async::Ready(()))
        }

        fn close(&mut self) -> Poll<(), io::Error> {
            Ok(Async::Ready(()))
        }
    }
    impl<TIn, TOut> Transport for MockTransport<TIn, TOut>
    where
        TIn: Debug,
        TIn: 'static,
        TOut: 'static,
    {
    }
}
