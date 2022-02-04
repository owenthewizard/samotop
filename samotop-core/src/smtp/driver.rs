#[cfg(feature = "driver")]
use crate::common::io::*;
use crate::common::*;
use crate::io::tls::MayBeTls;

use crate::smtp::*;

pub trait Drive: fmt::Debug {
    fn drive<'a, 'i, 'x, 's, 'f>(
        &'a self,
        io: &'i mut Box<dyn MayBeTls>,
        interpretter: &'x (dyn Interpret + Send + Sync),
        state: &'s mut SmtpContext,
    ) -> S1Fut<'f, std::result::Result<(), DriverError>>
    where
        'a: 'f,
        'i: 'f,
        'x: 'f,
        's: 'f;
}

#[cfg(feature = "driver")]
#[derive(Debug)]
pub struct SmtpDriver;

#[cfg(feature = "driver")]
impl Drive for SmtpDriver {
    fn drive<'a, 'i, 'x, 's, 'f>(
        &'a self,
        bare_io: &'i mut Box<dyn MayBeTls>,
        interpretter: &'x (dyn Interpret + Send + Sync),
        state: &'s mut SmtpContext,
    ) -> S1Fut<'f, std::result::Result<(), DriverError>>
    where
        'a: 'f,
        'i: 'f,
        'x: 'f,
        's: 'f,
    {
        Box::pin(async move {
            state.service().prepare_session(bare_io, state).await;
            let mut io = async_std::io::BufReader::new(bare_io);
            // fetch and apply commands
            loop {
                // write all pending responses
                while let Some(response) = state.session.pop_control() {
                    trace!("Processing driver control {:?}", response);
                    use async_std::io::prelude::WriteExt;
                    match response {
                        DriverControl::Response(bytes) => {
                            let writer = io.get_mut();
                            let write = writer
                                .write_all(bytes.as_ref())
                                .await
                                .map_err(DriverError::WriteFailed);
                            let flush = writer.flush().await.map_err(DriverError::WriteFailed);
                            match write.and(flush) {
                                Ok(()) => {}
                                Err(e) => {
                                    return Err(e);
                                }
                            }
                        }
                        DriverControl::Shutdown => {
                            // CHECKME: why?
                            state.session.input.extend_from_slice(io.buffer());
                            // TODO: replace with close() after https://github.com/async-rs/async-std/issues/977
                            match poll_fn(move |cx| Pin::new(io.get_mut()).poll_close(cx)).await {
                                Ok(()) => {
                                    trace!("Shutdown completed");
                                    return Ok(());
                                }
                                Err(e) => {
                                    return Err(DriverError::CloseFailed(e));
                                }
                            }
                        }
                        DriverControl::StartTls => {
                            Pin::new(io.get_mut()).encrypt();
                        }
                    }
                }

                match interpretter.interpret(state).await {
                    Ok(None) => {
                        // Action taken, but no input consumed (i.e. session setup / shut down)
                    }
                    Ok(Some(consumed)) => {
                        assert_ne!(consumed, 0, "If consumed is 0, infinite loop is likely.");
                        assert!(
                            consumed <= state.session.input.len(),
                            "The interpreter consumed more than a buffer? How?"
                        );
                        // TODO: handle buffer more efficiently, now allocating all the time
                        state.session.input = state.session.input.split_off(consumed);
                    }
                    Err(ParseError::Incomplete) => {
                        use async_std::io::prelude::BufReadExt;
                        // TODO: take care of large chunks without LF
                        match io.read_until(b'\n', &mut state.session.input).await {
                            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                                warn!("session read timeout");
                                state.session.say_shutdown_timeout();
                            }
                            Err(e) => return Err(e.into()),
                            Ok(0) => {
                                if state.session.input.is_empty() {
                                    // client went silent, we're done!
                                    state.session.shutdown();
                                } else {
                                    error!(
                                        "Incomplete and finished: {:?}",
                                        String::from_utf8_lossy(state.session.input.as_slice())
                                    );
                                    // client did not finish the command and left.
                                    state
                                        .session
                                        .say_shutdown_processing_err("Incomplete command".into());
                                };
                            }
                            Ok(_) => { /* good, interpret again */ }
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Invalid command {:?} - {}",
                            String::from_utf8_lossy(state.session.input.as_slice()),
                            e
                        );

                        // remove one line from the buffer
                        let split = state
                            .session
                            .input
                            .iter()
                            .position(|b| *b == b'\n')
                            .map(|p| p + 1)
                            .unwrap_or(state.session.input.len());
                        state.session.input = state.session.input.split_off(split);

                        if split == 0 {
                            warn!("Parsing failed on empty input, this will fail again, stopping the session");
                            state.session.say_shutdown_service_err()
                        } else {
                            state.session.say_invalid_syntax();
                        }
                    }
                };
            }
        })
    }
}

#[derive(Debug)]
pub enum DriverError {
    IoClosed,
    WriteFailed(io::Error),
    CloseFailed(io::Error),
    ParsingFailed(io::Error),
    IoFailed(io::Error),
}
impl fmt::Display for DriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for DriverError {}
impl From<std::io::Error> for DriverError {
    fn from(e: std::io::Error) -> Self {
        DriverError::IoFailed(e)
    }
}
