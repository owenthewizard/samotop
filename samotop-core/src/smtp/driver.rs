use crate::common::io::{prelude::BufReadExt, BufReader};
use crate::common::*;
use crate::io::tls::MayBeTls;
use crate::smtp::{DriverControl, Interpret, ParseError, SmtpState};
use async_std::prelude::FutureExt;
use std::fmt;
use std::fmt::Display;
use std::time::{Duration, Instant};

pub trait Drive {
    fn drive<'a, 's, 'f>(
        &'a mut self,
        state: &'s mut SmtpState,
    ) -> S1Fut<'f, std::result::Result<(), DriverError>>
    where
        'a: 'f,
        's: 'f;
}

pub struct SmtpDriver<IO> {
    /// the underlying IO, such as TcpStream
    /// It will be set to None once closed
    io: Option<BufReader<IO>>,
    buffer: Vec<u8>,
}

impl<IO> Drive for SmtpDriver<IO>
where
    IO: MayBeTls,
{
    fn drive<'a, 's, 'f>(
        &'a mut self,
        state: &'s mut SmtpState,
    ) -> S1Fut<'f, std::result::Result<(), DriverError>>
    where
        'a: 'f,
        's: 'f,
    {
        // default to a day?
        let read_timeout = state
            .service
            .read_timeout()
            .unwrap_or_else(|| Duration::from_secs(60 * 60 * 24));

        Box::pin(async move {
            while self.is_open() {
                // fetch and apply commands
                self.drive_once(state, read_timeout).await?
            }
            Ok(())
        })
    }
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
    fn is_open(&self) -> bool {
        self.io.is_some()
    }
    async fn drive_once(
        &mut self,
        state: &mut SmtpState,
        read_timeout: Duration,
    ) -> std::result::Result<(), DriverError> {
        let mut io = if let Some(io) = self.io.take() {
            io
        } else {
            return Err(DriverError::IoClosed);
        };

        // write all pending responses
        while let Some(response) = state.pop_control() {
            trace!("Processing driver control {:?}", response);
            match response {
                DriverControl::Response(bytes) => {
                    let writer = io.get_mut();
                    let write = writer
                        .write_all(bytes.as_ref())
                        .await
                        .map_err(|e| DriverError::WriteFailed(Box::new(e)));
                    let flush = writer
                        .flush()
                        .await
                        .map_err(|e| DriverError::WriteFailed(Box::new(e)));
                    match write.and(flush) {
                        Ok(()) => {}
                        Err(e) => {
                            return Err(e);
                        }
                    }
                }
                DriverControl::Shutdown => match io.get_mut().close().await {
                    Ok(()) => {
                        trace!("Shutdown completed");
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
            match state
                .service
                .get_interpretter()
                .interpret(self.buffer.as_slice(), state)
                .await
            {
                Ok(None) => {
                    // Action taken, but no input consumed (i.e. session setup / shut down)
                    break Some(io);
                }
                Ok(Some(consumed)) => {
                    assert!(consumed != 0, "if consumed is 0, infinite loop is likely");
                    // TODO: handle buffer more efficiently, now allocating all the time
                    self.buffer = self.buffer.split_off(consumed);
                    state.session.last_command_at = Instant::now();
                    break Some(io);
                }
                Err(ParseError::Incomplete) => {
                    // TODO: take care of large chunks without LF
                    match io
                        .read_until(b'\n', &mut self.buffer)
                        .timeout(read_timeout)
                        .await
                        .ok()
                        .transpose()?
                    {
                        None => {
                            warn!("session read timeout");
                            state.say_shutdown_timeout();
                        }
                        Some(0) => {
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
                        Some(_) => { /* good, interpret again */ }
                    }
                }
                Err(e) => {
                    warn!(
                        "Invalid command {:?} - {}",
                        String::from_utf8_lossy(self.buffer.as_slice()),
                        e
                    );

                    // remove one line from the buffer
                    let split = self
                        .buffer
                        .iter()
                        .position(|b| *b == b'\n')
                        .map(|p| p + 1)
                        .unwrap_or(self.buffer.len());
                    self.buffer = self.buffer.split_off(split);

                    if split == 0 {
                        warn!("Parsing failed on empty input, this will fail again, stopping the session");
                        state.say_shutdown_service_err()
                    } else {
                        state.say_invalid_syntax();
                    }

                    break Some(io);
                }
            };
        };

        Ok(())
    }
}

impl<IO> SmtpDriver<IO> {
    pub fn into_inner(self) -> (Vec<u8>, Option<IO>) {
        let Self { io, buffer } = self;
        (buffer, io.map(|io| io.into_inner()))
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
