use crate::common::*;
use crate::io::tls::MayBeTls;
use crate::parser::Interpret;
use crate::smtp::CodecControl;
use crate::smtp::SessionShutdown;
use crate::smtp::SmtpState;
use crate::{
    parser::ParseError,
    smtp::{ProcessingError, SmtpSessionCommand},
};
use futures_util::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use std::fmt;

pub struct SmtpDriver<IO> {
    /// the underlying IO, such as TcpStream
    /// It will be set to None once closed
    io: Option<BufReader<IO>>,
    buffer: Vec<u8>,
}

impl<IO> SmtpDriver<IO>
where
    IO: MayBeTls,
{
    pub fn new(io: IO) -> Self {
        SmtpDriver {
            io: Some(BufReader::new(io)),
            buffer: vec![],
        }
    }
    pub fn is_open(&self) -> bool {
        self.io.is_some()
    }
    pub async fn drive(
        &mut self,
        interpretter: &(dyn Interpret + Sync),
        state: &mut SmtpState,
    ) -> Result<()> {
        let mut io = if let Some(io) = self.io.take() {
            io
        } else {
            return Err(todo!("closed"));
        };

        // write all pending responses
        while let Some(response) = state.writes.pop_front() {
            trace!("Processing codec control {:?}", response);
            match response {
                CodecControl::Parser(newparser) => unimplemented!(),
                CodecControl::Response(bytes) => {
                    match io.write_all(bytes.as_ref()).await {
                        Ok(()) => {
                            // all good, carry on
                        }
                        Err(e) => {
                            state.writes.push_front(CodecControl::Response(bytes));
                            return Err(processing_error("Write failed", e));
                        }
                    }
                }
                CodecControl::Shutdown => match io.close().await {
                    Ok(()) => {
                        trace!("Close complete");
                        //io stays None
                        return Ok(());
                    }
                    Err(e) => {
                        return Err(processing_error("Close failed", e));
                    }
                },
                CodecControl::StartTls => {
                    Pin::new(io.get_mut()).encrypt();
                }
            }
        }

        self.io = loop {
            match interpretter.interpret(self.buffer.as_slice(), state).await {
                Ok(consumed) => {
                    assert!(consumed != 0, "if consumed is 0, infinite loop is likely");
                    // TODO: handle buffer more efficiently, now allocating all the time
                    self.buffer = self.buffer.split_off(consumed);
                    break Some(io);
                }
                Err(ParseError::Incomplete) => {
                    // TODO: take care of large chunks without LF
                    match io.read_until(b'\n', &mut self.buffer).await? {
                        0 => {
                            let s = std::mem::take(state);
                            *state = if self.buffer.is_empty() {
                                // client went silent, we're done!
                                SessionShutdown.apply(s).await
                            } else {
                                error!(
                                    "Incomplete and finished: {:?}",
                                    String::from_utf8_lossy(self.buffer.as_slice())
                                );
                                // client did not finish the command and left.
                                ProcessingError.apply(s).await
                            };
                            break Some(io);
                        }
                        _ => {}
                    }
                }
                Err(e) => return Err(processing_error("Parsing failed", e)),
            };
        };

        Ok(())
    }
}

fn processing_error(scope: &str, e: impl fmt::Debug) -> Error {
    todo!()
}
