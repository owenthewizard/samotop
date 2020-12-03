use crate::common::*;
use crate::io::tls::MayBeTls;
use crate::smtp::CodecControl;
use async_std::io::BufReader;
use samotop_model::{
    parser::{DummyParser, ParseError, Parser},
    smtp::{ProcessingError, SmtpSessionCommand},
};
use std::{collections::VecDeque, fmt};

pub struct SmtpCodec<IO> {
    /// the underlying IO, such as TcpStream
    io: Option<BufReader<IO>>,
    /// server to client encoded responses buffer
    s2c_pending: VecDeque<CodecControl>,
    /// current parser to use for input data
    parser: Box<dyn Parser + Sync + Send>,
}

impl<IO> Stream for SmtpCodec<IO>
where
    IO: MayBeTls,
{
    type Item = Box<dyn SmtpSessionCommand>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let SmtpCodec {
            io,
            s2c_pending,
            parser,
            ..
        } = self.get_mut();

        while let Some(write) = s2c_pending.pop_front() {
            let writer = match io.as_mut() {
                None => continue,
                Some(io) => Pin::new(io.get_mut()),
            };
            trace!("Processing codec control {:?}", write);
            match write {
                CodecControl::Parser(newparser) => *parser = newparser,
                CodecControl::Response(bytes) => {
                    match writer.poll_write(cx, bytes.as_ref()) {
                        Poll::Ready(Ok(len)) if len == bytes.len() => {
                            // all good, carry on
                        }
                        Poll::Ready(Ok(len)) => {
                            // partially written, let's try again...
                            s2c_pending.push_front(CodecControl::Response(bytes[len..].to_vec()));
                        }
                        Poll::Pending => {
                            s2c_pending.push_front(CodecControl::Response(bytes));
                            return Poll::Pending;
                        }
                        Poll::Ready(Err(e)) => {
                            s2c_pending.push_front(CodecControl::Response(bytes));
                            return Poll::Ready(Some(processing_error("Write failed", e)));
                        }
                    }
                }
                CodecControl::Shutdown => match writer.poll_close(cx) {
                    Poll::Ready(Ok(())) => {
                        trace!("Close complete");
                        *io = None;
                    }
                    Poll::Ready(Err(e)) => {
                        *io = None;
                        return Poll::Ready(Some(processing_error("Close failed", e)));
                    }
                    Poll::Pending => {
                        trace!("Close pending");
                        s2c_pending.push_front(CodecControl::Shutdown);
                    }
                },
                CodecControl::StartTls => {
                    writer.encrypt();
                }
            }
        }

        loop {
            let mut reader = match io.as_mut() {
                None => break Poll::Ready(None),
                Some(io) => Pin::new(io),
            };
            break match parser.parse_command(reader.buffer()) {
                Ok((i, cmd)) => {
                    let consumed = reader.buffer().len() - i.len();
                    reader.as_mut().consume(consumed);
                    Poll::Ready(Some(cmd))
                }
                Err(ParseError::Incomplete) => {
                    let len = reader.buffer().len();
                    match ready!(reader.as_mut().poll_fill_buf(cx)) {
                        Ok(new) if new.len() == len || new.is_empty() => {
                            // BufReader::poll_fill_buf() only works on empty buffer
                            break Poll::Ready(Some(processing_error(
                                "Incomplete and finished",
                                String::from_utf8_lossy(new),
                            )));
                        }
                        Ok(_) => continue,
                        Err(e) => break Poll::Ready(Some(processing_error("Read failed", e))),
                    }
                }
                Err(e) => Poll::Ready(Some(processing_error("Parsing failed", e))),
            };
        }
    }
}

impl<IO> SmtpCodec<IO>
where
    IO: MayBeTls,
{
    pub fn send(&mut self, control: CodecControl) {
        self.s2c_pending.push_back(control)
    }
}

impl<IO: MayBeTls> SmtpCodec<IO> {
    pub fn new(io: IO) -> Self {
        SmtpCodec {
            io: Some(BufReader::new(io)),
            parser: Box::new(DummyParser),
            s2c_pending: vec![].into(),
        }
    }
}

fn processing_error(scope: &str, e: impl fmt::Debug) -> Box<dyn SmtpSessionCommand> {
    error!("{}: {:?}", scope, e);
    Box::new(ProcessingError::default())
}
