use std::io;
use std::fmt;
use std::io::{Read, Write};
use bytes::Bytes;
use futures::sync::mpsc::Sender;
use futures::{Stream, Sink, Async, AsyncSink, StartSend, Poll, Future};
use tokio_proto::streaming::pipeline::Transport;
use model::response::SmtpReply;
use model::machine::{start, SmtpMachine};
use model::request::*;
use model::act;
use model::state;

#[derive(Debug)]
struct Cradle<T> {
    item: Option<T>,
}
impl<T> Cradle<T> {
    pub fn turn<R, F: FnOnce(T) -> (T, R)>(&mut self, f: F) -> R {
        let (next, result) = f(self.item.take().unwrap());
        self.item = Some(next);
        result
    }
    pub fn touch<R, F: FnOnce(&mut T) -> R>(&mut self, f: F) -> R {
        let mut m = self.item.take().unwrap();
        let result = f(&mut m);
        self.item = Some(m);
        result
    }
    pub fn into(self) -> T {
        self.item.unwrap()
    }
}
impl<T> From<T> for Cradle<T> {
    fn from(item: T) -> Self {
        Self { item: Some(item) }
    }
}

enum State<D> {
    New(SmtpMachine<state::Session, D>),
    Connected(SmtpMachine<state::Conn, D>),
    Initialized(SmtpMachine<state::Helo, D>),
    MailFrom(SmtpMachine<state::Mail, D>),
    RcptTo(SmtpMachine<state::Rcpt, D>),
    Streaming(SmtpMachine<state::Data, D>),
    Ready(SmtpMachine<act::Mail, D>),
    Closed(SmtpMachine<state::Closed, D>),
}

impl<D> State<D> {
    fn name<'a>(&'a self) -> &'a fmt::Display {
        use self::State::*;
        match self {
            &New(_) => &"New",
            &Connected(_) => &"Connected",
            &Initialized(_) => &"Helo",
            &MailFrom(_) => &"MailFrom",
            &RcptTo(_) => &"RcptTo",
            &Streaming(_) => &"Data",
            &Ready(_) => &"Ready",
            &Closed(_) => &"Closed",
        }
    }
    fn machine<'a>(&'a self) -> &'a fmt::Debug {
        use self::State::*;
        match self {
            &New(ref m) => m,
            &Connected(ref m) => m,
            &Initialized(ref m) => m,
            &MailFrom(ref m) => m,
            &RcptTo(ref m) => m,
            &Streaming(ref m) => m,
            &Ready(ref m) => m,
            &Closed(ref m) => m,
        }
    }
}

impl<D> fmt::Debug for State<D> {
    fn fmt<'a>(&self, fmt: &mut fmt::Formatter<'a>) -> Result<(), fmt::Error> {
        fmt.write_fmt(format_args!("{}({:?})", self.name(), self.machine()))
    }
}

impl<D> Stream for State<D>
where
    D: Sink<SinkItem = SmtpReply, SinkError = io::Error>,
    D: Stream<Item = SmtpInput, Error = io::Error>,
{
    type Item = SmtpInput;
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        use self::State::*;
        match self {
            &mut New(ref mut m) => m.poll(),
            &mut Connected(ref mut m) => m.poll(),
            &mut Initialized(ref mut m) => m.poll(),
            &mut MailFrom(ref mut m) => m.poll(),
            &mut RcptTo(ref mut m) => m.poll(),
            &mut Streaming(ref mut m) => m.poll(),
            &mut Ready(ref mut m) => m.poll(),
            &mut Closed(ref mut m) => m.poll(),
        }
    }
}

impl<D> From<SmtpMachine<state::Session, D>> for State<D> {
    fn from(machine: SmtpMachine<state::Session, D>) -> Self {
        State::New(machine)
    }
}
impl<D> From<SmtpMachine<state::Conn, D>> for State<D> {
    fn from(machine: SmtpMachine<state::Conn, D>) -> Self {
        State::Connected(machine)
    }
}
impl<D> From<SmtpMachine<state::Helo, D>> for State<D> {
    fn from(machine: SmtpMachine<state::Helo, D>) -> Self {
        State::Initialized(machine)
    }
}
impl<D> From<SmtpMachine<state::Mail, D>> for State<D> {
    fn from(machine: SmtpMachine<state::Mail, D>) -> Self {
        State::MailFrom(machine)
    }
}
impl<D> From<SmtpMachine<state::Rcpt, D>> for State<D> {
    fn from(machine: SmtpMachine<state::Rcpt, D>) -> Self {
        State::RcptTo(machine)
    }
}
impl<D> From<SmtpMachine<state::Data, D>> for State<D> {
    fn from(machine: SmtpMachine<state::Data, D>) -> Self {
        State::Streaming(machine)
    }
}
impl<D> From<SmtpMachine<act::Mail, D>> for State<D> {
    fn from(machine: SmtpMachine<act::Mail, D>) -> Self {
        State::Ready(machine)
    }
}
impl<D> From<SmtpMachine<state::Closed, D>> for State<D> {
    fn from(machine: SmtpMachine<state::Closed, D>) -> Self {
        State::Closed(machine)
    }
}

struct Null;
impl Write for Null {
    fn write(&mut self, bytes: &[u8]) -> Result<usize, io::Error> {
        Ok(0)
    }
    fn flush(&mut self) -> Result<(), io::Error> {
        Ok(())
    }
}
impl Read for Null {
    fn read(&mut self, bytes: &mut [u8]) -> Result<usize, io::Error> {
        Ok(0)
    }
}

pub struct ActTransport<TT> {
    machine: Cradle<State<TT>>,
}

impl<TT> ActTransport<TT> {
    pub fn new(upstream: TT) -> ActTransport<TT> {
        ActTransport { machine: State::New(start(upstream)).into() }
    }
    fn animate<D>(machine: State<D>, inp: SmtpInput) -> State<D> {
        use self::SmtpInput::*;
        use self::SmtpCommand::*;
        use self::State::*;
        match machine {
            New(m) => {
                match inp {
                    Connect(c) => m.connect(c).into(),
                    _ => m.unexpected(inp).into(),
                }
            }
            Connected(m) => {
                match inp {
                    Command(_, _, Helo(c)) => m.helo(c).into(),
                    _ => m.unexpected(inp).into(),
                }
            }
            Initialized(m) => {
                match inp {
                    Command(_, _, Mail(c)) => m.mail(c).into(),
                    _ => m.unexpected(inp).into(),
                }
            }
            MailFrom(m) => {
                match inp {
                    Command(_, _, Rcpt(c)) => m.rcpt(c).into(),
                    _ => m.unexpected(inp).into(),
                }
            }
            RcptTo(m) => {
                match inp {
                    Command(_, _, Data) => m.data(Box::new(Null), Box::new(Null)).into(),
                    _ => m.unexpected(inp).into(),
                }
            }
            Streaming(m) => {
                match inp {
                    StreamEnd(_) => m.done().into(),
                    _ => m.unexpected(inp).into(),
                }
            }
            Ready(m) => panic!("special case"),
            Closed(m) => panic!("closed"),
        }
    }
}


impl<TT> Stream for ActTransport<TT>
where
    TT: 'static + Stream<Item = SmtpInput, Error = io::Error>,
    TT: Sink<SinkItem = SmtpReply, SinkError = io::Error>,
{
    type Error = io::Error;
    type Item = act::Act;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        trace!("upstream poll...");
        // Poll the upstream transport. `try_ready!` will bubble up
        // errors and Async::NotReady.
        let poll = self.machine.touch(|mut m| m.poll());
        trace!("upstream poll result: {:?}", poll);
        if let Some(inp) = try_ready!(poll) {
            trace!("Got {:?} for {:?}", inp, self.machine);
            self.machine.turn(
                |m| (Self::animate(m, inp), Ok(Async::NotReady)),
            )
        } else {
            trace!("Got None");
            Ok(Async::Ready(None))
        }
    }
}

impl<TT> Sink for ActTransport<TT>
where
    TT: 'static + Stream<Item = SmtpInput, Error = io::Error>,
    TT: Sink<SinkItem = SmtpReply, SinkError = io::Error>,
{
    type SinkError = io::Error;
    type SinkItem = act::ActResult;

    fn start_send(&mut self, rsp: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        self.machine.turn(|s| {
            (
                match s {
                    State::Ready(m) => {
                        match rsp {
                            Err(_) => m.failed().into(),
                            Ok(_) => m.ok().into(),
                        }
                    }
                    _ => s,
                },
                Ok(AsyncSink::Ready),
            )
        })
    }

    fn poll_complete(&mut self) -> Poll<(), io::Error> {
        Ok(Async::Ready(()))
    }

    fn close(&mut self) -> Poll<(), io::Error> {
        Ok(Async::Ready(()))
    }
}

impl<TT> Transport for ActTransport<TT>
where
    TT: 'static,
    TT: Stream<Error = io::Error, Item = SmtpInput>,
    TT: Sink<SinkError = io::Error, SinkItem = SmtpReply>,
{
}
