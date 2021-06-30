use crate::common::*;
use crate::io::tls::MayBeTls;
use crate::smtp::CodecControl;
use crate::{
    parser::{ParseError, Parser},
    smtp::{ProcessingError, SmtpSessionCommand},
};
use bytes::{Buf, BufMut, BytesMut};
use std::{collections::VecDeque, fmt};

pub struct SmtpCodec<IO> {
    /// the underlying IO, such as TcpStream
    io: Option<IO>,
    buffer: BytesMut,
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
            buffer,
        } = self.get_mut();

        while let Some(write) = s2c_pending.pop_front() {
            let writer = match io.as_mut() {
                None => continue,
                Some(io) => Pin::new(io),
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
            let reader = match io.as_mut() {
                None => break Poll::Ready(None),
                Some(io) => Pin::new(io),
            };
            break match parser.parse_command(buffer.chunk()) {
                Ok((i, cmd)) => {
                    let consumed = buffer.chunk().len() - i.len();
                    buffer.advance(consumed);
                    Poll::Ready(Some(cmd))
                }
                Err(ParseError::Incomplete) => {
                    if buffer.remaining_mut() == 0 {
                        buffer.reserve(1024);
                    }
                    let buff = buffer.chunk_mut();
                    // This is unsafe because BytesMut does not initialize the buffer.
                    // Malicious reader could get access to random / interesting data!
                    // Accepting unsafe here we assume the reader is not malicious and
                    // only writes, doesn't read the buffer
                    // TODO: check clippy::transmute-ptr-to-ptr complaint
                    #[allow(clippy::transmute_ptr_to_ptr)]
                    let buff = unsafe { std::mem::transmute(buff) };
                    match ready!(reader.poll_read(cx, buff)) {
                        Ok(0) => Poll::Ready(Some(processing_error(
                            "Incomplete and finished",
                            String::from_utf8_lossy(buffer.chunk()),
                        ))),
                        Ok(len) => {
                            // This is unsafe because badly behaved reader could return a different
                            // len than actually written into the buffer or write at an offset.
                            // This would cause us to receive random data and treat it as SMTP input!
                            // Accepting unsafe here we assume that the reader is well behaved and works correctly.
                            unsafe { buffer.advance_mut(len) };
                            continue;
                        }
                        Err(e) => Poll::Ready(Some(processing_error("Read failed", e))),
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
            io: Some(io),
            buffer: BytesMut::default(),
            parser: Box::new(()),
            s2c_pending: vec![].into(),
        }
    }
}

fn processing_error(scope: &str, e: impl fmt::Debug) -> Box<dyn SmtpSessionCommand> {
    error!("{}: {:?}", scope, e);
    Box::new(ProcessingError::default())
}
