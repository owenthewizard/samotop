use crate::common::*;
use crate::io::tls::MayBeTls;
use crate::smtp::{Interpret, ParseError, SmtpState};
use futures_util::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::fmt;
use std::fmt::Display;

/// Represents the instructions for the client side of the stream.
pub enum DriverControl {
    /// Write an SMTP response
    Response(Vec<u8>),
    /// Start TLS encryption
    StartTls,
    /// Shut the stream down
    Shutdown,
}

impl fmt::Debug for DriverControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[derive(Debug)]
        enum TextOrBytes<'a> {
            T(&'a str),
            B(&'a [u8]),
        }
        fn tb(inp: &[u8]) -> TextOrBytes {
            if let Ok(text) = std::str::from_utf8(inp) {
                TextOrBytes::T(text)
            } else {
                TextOrBytes::B(inp)
            }
        }
        match self {
            DriverControl::Response(r) => f.debug_tuple("Response").field(&tb(r)).finish(),
            DriverControl::StartTls => f.debug_tuple("StartTls").finish(),
            DriverControl::Shutdown => f.debug_tuple("Shutdown").finish(),
        }
    }
}

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
    ) -> std::result::Result<(), DriverError> {
        let mut io = if let Some(io) = self.io.take() {
            io
        } else {
            return Err(DriverError::IoClosed);
        };

        // write all pending responses
        while let Some(response) = state.writes.pop_front() {
            trace!("Processing codec control {:?}", response);
            match response {
                DriverControl::Response(bytes) => {
                    match io.write_all(bytes.as_ref()).await {
                        Ok(()) => {
                            // all good, carry on
                        }
                        Err(e) => {
                            state.writes.push_front(DriverControl::Response(bytes));
                            return Err(DriverError::WriteFailed(Box::new(e)));
                        }
                    }
                }
                DriverControl::Shutdown => match io.close().await {
                    Ok(()) => {
                        trace!("Close complete");
                        //io stays None
                        return Ok(());
                    }
                    Err(e) => {
                        return Err(DriverError::CloseFailed(Box::new(e)));
                    }
                },
                DriverControl::StartTls => {
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
                    if io.read_until(b'\n', &mut self.buffer).await? == 0 {
                        if self.buffer.is_empty() {
                            // client went silent, we're done!
                            state.shutdown();
                        } else {
                            error!(
                                "Incomplete and finished: {:?}",
                                String::from_utf8_lossy(self.buffer.as_slice())
                            );
                            // client did not finish the command and left.
                            state.say_shutdown_processing_err("Incomplete command".into());
                        };
                        break Some(io);
                    }
                }
                Err(e) => return Err(DriverError::ParsingFailed(Box::new(e))),
            };
        };

        Ok(())
    }
}

#[derive(Debug)]
pub enum DriverError {
    IoClosed,
    WriteFailed(Error),
    CloseFailed(Error),
    ParsingFailed(Error),
    IoFailed(Error),
}
impl Display for DriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for DriverError {}
impl From<std::io::Error> for DriverError {
    fn from(e: std::io::Error) -> Self {
        DriverError::IoFailed(Box::new(e))
    }
}
