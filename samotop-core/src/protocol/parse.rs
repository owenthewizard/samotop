use crate::common::*;
use crate::parser::Parser;
use crate::smtp::ReadControl;
use memchr::memchr;
use samotop_model::{parser::ParseError, smtp::SmtpSessionCommand};

pub trait IntoParse
where
    Self: Sized,
{
    fn parse<P>(self, parser: P) -> Parse<Self, P> {
        Parse::new(self, parser)
    }
}

impl<S> IntoParse for S where S: Stream<Item = Result<ReadControl>> {}

#[pin_project(project=ParseProjection)]
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct Parse<S, P> {
    #[pin]
    stream: S,
    parser: P,
    bytes: Vec<u8>,
    pending: Option<Result<ReadControl>>,
}

impl<S, P> Parse<S, P> {
    pub fn new(stream: S, parser: P) -> Self {
        Self {
            stream,
            parser,
            bytes: vec![],
            pending: None,
        }
    }
}

impl<S, P> Stream for Parse<S, P>
where
    S: Stream<Item = Result<ReadControl>>,
    P: Parser,
{
    type Item = S::Item;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let ParseProjection {
            mut stream,
            parser,
            bytes,
            pending,
        } = self.project();
        loop {
            if !bytes.is_empty() {
                match parser.command(bytes.as_ref()) {
                    Ok((remaining, command)) => {
                        let len = bytes.len() - remaining.len();
                        let remaining = bytes.split_off(len);
                        let current = std::mem::replace(bytes, remaining);
                        trace!("Parsed {} bytes - a {} command", len, command.verb());
                        return Poll::Ready(Some(Ok(ReadControl::Command(
                            Box::new(command),
                            current,
                        ))));
                    }
                    Err(ParseError::Mismatch(_)) => {
                        let len = memchr(b'\n', bytes.as_ref()).unwrap_or_else(|| bytes.len());
                        let remaining = bytes.split_off(len);
                        let current = std::mem::replace(bytes, remaining);
                        warn!("Parser did not match, passing current line as is: {}.", len);
                        return Poll::Ready(Some(Ok(ReadControl::Raw(current))));
                    }
                    Err(ParseError::Failed(e)) => {
                        return Poll::Ready(Some(Err(
                            format!("Parsing command failed: {}", e).into()
                        )));
                    }
                    Err(ParseError::Incomplete) => {
                        // will need more bytes...
                    }
                }
            }

            if pending.is_none() {
                // we got here looking for more.
                // it's either in the buffer already or we ask for it
                *pending = ready!(stream.as_mut().poll_next(cx));
            }

            match pending.take() {
                Some(Ok(ReadControl::Raw(new))) => {
                    if bytes.is_empty() {
                        *bytes = new;
                    } else {
                        bytes.extend_from_slice(new.as_slice());
                    }
                    // we got some bytes, let's munch in next loop round!
                    continue;
                }
                other => {
                    if bytes.is_empty() {
                        trace!("Passing server control {:?}", other);
                        return Poll::Ready(other);
                    } else {
                        trace!("Passing server control {:?} after the tail", other);
                        // we can't process this one until we deal with the bytes.
                        // we'll put it in the pending buffer for next call.
                        *pending = other;
                        let current = std::mem::take(bytes);
                        return Poll::Ready(Some(Ok(ReadControl::Raw(current))));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod parse_tests {
    use crate::parser::Parser;
    use crate::smtp::ReadControl::*;
    use crate::smtp::SmtpCommand::{self, *};
    use crate::test_util::*;

    use super::*;

    struct FakeParser<T>(T);
    impl Parser for FakeParser<(SmtpCommand, &'static [u8])> {
        fn command(&self, input: &[u8]) -> Result<SmtpCommand> {
            if input == (self.0).1 {
                Ok((self.0).0.clone())
            } else {
                Err("incomplete or mismatch".into())
            }
        }
        fn script(&self, input: &[u8]) -> Result<Vec<ReadControl>> {
            if input == (self.0).1 {
                Ok(vec![ReadControl::Command(
                    self.0 .0.clone(),
                    Vec::from(input),
                )])
            } else {
                Err("incomplete or mismatch".into())
            }
        }
        fn forward_path(&self, _input: &[u8]) -> Result<samotop_model::smtp::SmtpPath> {
            unimplemented!()
        }
    }
    impl Parser for FakeParser<ReadControl> {
        fn command(&self, _input: &[u8]) -> Result<SmtpCommand> {
            if let ReadControl::Command(c, _) = self.0.clone() {
                Ok(c)
            } else {
                Err("wrong".into())
            }
        }
        fn script(&self, _input: &[u8]) -> Result<Vec<ReadControl>> {
            Ok(vec![self.0.clone()])
        }
        fn forward_path(&self, _input: &[u8]) -> Result<samotop_model::smtp::SmtpPath> {
            unimplemented!()
        }
    }
    impl Parser for FakeParser<Vec<ReadControl>> {
        fn command(&self, _input: &[u8]) -> Result<SmtpCommand> {
            Err("wrong".into())
        }
        fn script(&self, _input: &[u8]) -> Result<Vec<ReadControl>> {
            Ok(self.0.clone())
        }
        fn forward_path(&self, _input: &[u8]) -> Result<samotop_model::smtp::SmtpPath> {
            unimplemented!()
        }
    }
    impl Parser for FakeParser<()> {
        fn command(&self, _input: &[u8]) -> Result<SmtpCommand> {
            Err("fail".into())
        }
        fn script(&self, _input: &[u8]) -> Result<Vec<ReadControl>> {
            Err("fail".into())
        }
        fn forward_path(&self, _input: &[u8]) -> Result<samotop_model::smtp::SmtpPath> {
            unimplemented!()
        }
    }

    #[test]
    fn poll_next_handles_partial_input_with_pending() -> Result<()> {
        let setup = TestStream::from(vec![Poll::Ready(Some(Ok(Raw(b("uhu"))))), Poll::Pending]);
        let mut sut = setup.parse(FakeParser(()));
        let res = Pin::new(&mut sut).poll_next(&mut cx());

        assert_eq!(res?, Poll::Pending);
        Ok(())
    }

    #[test]
    fn poll_next_handles_partial_input_with_concatenation() -> Result<()> {
        let setup = TestStream::from(vec![
            Poll::Ready(Some(Ok(Raw(b("qu"))))),
            Poll::Ready(Some(Ok(Raw(b("it"))))),
            Poll::Ready(Some(Ok(Raw(b("\r\n"))))),
        ]);
        let mut sut = setup.parse(FakeParser((Quit, "quit\r\n".as_bytes())));
        let res = Pin::new(&mut sut).poll_next(&mut cx());
        assert_eq!(res?, Poll::Ready(Some(Command(Quit, b("quit\r\n")))));
        Ok(())
    }

    #[test]
    fn poll_next_handles_pipelining() -> Result<()> {
        let setup = TestStream::from(vec![Poll::Ready(Some(Ok(Raw(b("quit\r\nquit\r\n")))))]);
        let mut sut = setup.parse(FakeParser(vec![
            Command(Quit, b("quit\r\n")),
            Command(Quit, b("quit\r\n")),
        ]));

        let res = Pin::new(&mut sut).poll_next(&mut cx());
        if let Poll::Ready(Some(Ok(Command(Quit, _)))) = res {
            //cool
        } else {
            panic!("Expected Quit command, got {:?}", res)
        }

        let res = Pin::new(&mut sut).poll_next(&mut cx());
        if let Poll::Ready(Some(Ok(Command(Quit, _)))) = res {
            //cool
        } else {
            panic!("Expected Quit command, got {:?}", res)
        }

        Ok(())
    }
}
