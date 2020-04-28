use crate::grammar::Parser;
use crate::model::command::SmtpCommand;
use crate::model::controll::ServerControll;

use tokio::prelude::*;

pub trait IntoParse
where
    Self: Sized,
{
    fn parse<P>(self, parser: P) -> Parse<Self, P> {
        Parse::new(self, parser)
    }
}

impl<S> IntoParse for S
where
    S: Stream,
{
}

pub struct Parse<S, P> {
    stream: S,
    parser: P,
}

impl<S, P> Parse<S, P> {
    pub fn new(stream: S, parser: P) -> Self {
        Self { stream, parser }
    }
}

impl<S, P> Stream for Parse<S, P>
where
    S: Stream<Item = ServerControll>,
    P: Parser,
{
    type Item = ServerControll;
    type Error = S::Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match try_ready!(self.stream.poll()) {
            Some(ServerControll::Command(SmtpCommand::Unknown(line))) => {
                match self.parser.command(&line) {
                    Ok(cmd) => Ok(Async::Ready(Some(ServerControll::Command(cmd)))),
                    _ => Ok(Async::Ready(Some(ServerControll::Command(
                        SmtpCommand::Unknown(line),
                    )))),
                }
            }
            pass => Ok(Async::Ready(pass)),
        }
    }
}
