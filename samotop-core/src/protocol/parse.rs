use crate::common::*;
use crate::parser::Parser;
use crate::smtp::ReadControl;

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
    input: Vec<Option<Result<ReadControl>>>,
}

impl<S, P> Parse<S, P> {
    pub fn new(stream: S, parser: P) -> Self {
        Self {
            stream,
            parser,
            input: vec![],
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
            input,
        } = self.project();
        loop {
            let tail = match input.first() {
                Some(Some(Ok(ReadControl::Raw(ref bytes)))) if !bytes.ends_with(b"\n") => {
                    // this is a line without LF, we'll concat with the next
                    if let Some(Ok(ReadControl::Raw(bytes))) = input.remove(0) {
                        Some(bytes)
                    } else {
                        unreachable!("checked in previous match")
                    }
                }
                _ => {
                    // it is not an open ended raw line, leave it put
                    None
                }
            };

            if !input.is_empty() {
                assert!(
                    tail.is_none(),
                    "In previous code block, tail is some only if it is the last element"
                );
                // return previously parsed items
                return Poll::Ready(input.remove(0));
            }

            match ready!(stream.as_mut().poll_next(cx)) {
                Some(Ok(ReadControl::Raw(mut bytes))) => {
                    if let Some(tail) = tail {
                        let mut bytes2 = tail.to_vec();
                        bytes2.extend_from_slice(&bytes[..]);
                        // concat previous open ended line with new raw
                        bytes = bytes2;
                    }

                    trace!("Parsing {} raw bytes as a script", bytes.len());
                    match parser.script(&bytes[..]) {
                        Ok(script) => {
                            trace!("Parsed a script of {} inputs", script.len());
                            input.extend(script.into_iter().map(|i| Some(Ok(i))))
                        }
                        _ => {
                            warn!("Parsing the script failed, passing as is.");
                            input.push(Some(Ok(ReadControl::Raw(bytes))));
                        }
                    }
                }
                other => {
                    if let Some(bytes) = tail {
                        trace!("Passing server control {:?} after the tail", other);
                        input.insert(0, other);
                        return Poll::Ready(Some(Ok(ReadControl::Raw(bytes))));
                    } else {
                        trace!("Passing server control {:?}", other);
                        return Poll::Ready(other);
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
