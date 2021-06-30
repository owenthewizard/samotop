use crate::common::*;
use crate::io::tls::MayBeTls;
use crate::parser::Interpret;
use crate::smtp::CodecControl;
use crate::smtp::SmtpState;
use crate::{
    parser::ParseError,
    smtp::{ProcessingError, SmtpSessionCommand},
};
use bytes::{Buf, BufMut};
use futures_util::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use std::fmt;

pub struct SmtpDriver<IO> {
    /// the underlying IO, such as TcpStream
    io: BufReader<IO>,
    buffer: Vec<u8>,
}

impl<IO> SmtpDriver<IO>
where
    IO: MayBeTls,
{
    pub fn new(io: IO) -> Self {
        SmtpDriver {
            io: BufReader::new(io),
            buffer: vec![],
        }
    }
    pub async fn drive(&mut self, interpret: &dyn Interpret, state: &mut SmtpState) -> Result<()> {
        // write all pending responses
        while let Some(response) = state.writes.pop_front() {
            trace!("Processing codec control {:?}", response);
            match response {
                CodecControl::Parser(newparser) => unimplemented!(),
                CodecControl::Response(bytes) => {
                    match self.io.write_all(bytes.as_ref()).await {
                        Ok(()) => {
                            // all good, carry on
                        }
                        Err(e) => {
                            state.writes.push_front(CodecControl::Response(bytes));
                            return Err(processing_error("Write failed", e));
                        }
                    }
                }
                CodecControl::Shutdown => match self.io.close().await {
                    Ok(()) => {
                        trace!("Close complete");
                    }
                    Err(e) => {
                        return Err(processing_error("Close failed", e));
                    }
                },
                CodecControl::StartTls => {
                    Pin::new(self.io.get_mut()).encrypt();
                }
            }
        }

        loop {
            break match interpret.interpret(self.buffer.as_slice(), state).await {
                Ok(consumed) => {
                    // TODO: handle buffer more efficiently, now allocating all the time
                    self.buffer = self.buffer.split_off(consumed);
                    continue;
                }
                Err(ParseError::Incomplete) => {
                    // TODO: take care of large chunks without LF
                    match self.io.read_until(b'\n', &mut self.buffer).await? {
                        0 => {
                            let s = std::mem::take(state);
                            *state = ProcessingError.apply(s).await;
                            Err(processing_error(
                                "Incomplete and finished",
                                String::from_utf8_lossy(self.buffer.as_slice()),
                            ))
                        }
                        _ => continue,
                    }
                }
                Err(e) => Err(processing_error("Parsing failed", e)),
            };
        }
    }
}

fn processing_error(scope: &str, e: impl fmt::Debug) -> Error {
    error!("{}: {:?}", scope, e);
    todo!()
}
