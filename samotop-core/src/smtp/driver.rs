use crate::common::io::{prelude::BufReadExt, BufReader};
use crate::common::*;
use crate::io::tls::MayBeTls;
use crate::smtp::{DriverControl, Interpret, ParseError, SmtpState};
use std::fmt;
use std::fmt::Display;

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
                    match io.get_mut().write_all(bytes.as_ref()).await {
                        Ok(()) => {
                            // all good, carry on
                        }
                        Err(e) => {
                            state.writes.push_front(DriverControl::Response(bytes));
                            return Err(DriverError::WriteFailed(Box::new(e)));
                        }
                    }
                }
                DriverControl::Shutdown => match io.get_mut().close().await {
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
                Err(e) => {
                    warn!(
                        "Invalid command {:?} - {}",
                        String::from_utf8_lossy(self.buffer.as_slice()),
                        e
                    );
                    state.say_invalid_syntax();
                    
                    // remove one line from the buffer
                    let split = self
                        .buffer
                        .iter()
                        .position(|b| *b == b'\n')
                        .map(|p| p + 1)
                        .unwrap_or(self.buffer.len());
                    self.buffer = self.buffer.split_off(split);
                    break Some(io);
                }
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
