use crate::{
    common::{io::Write, *},
    config::{ServerContext, Setup},
    io::{ConnectionInfo, Handler, HandlerService, Session},
    smtp::*,
};
use async_std::io::prelude::BufReadExt;
use async_std::io::BufReader;
use async_std::io::WriteExt;

#[derive(Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct SmtpDriver;

impl Setup for SmtpDriver {
    fn setup(&self, ctx: &mut ServerContext) {
        ctx.store.add::<HandlerService>(Arc::new(SmtpDriver))
    }
}

impl Handler for SmtpDriver {
    fn handle<'s, 'a, 'f>(&'s self, session: &'a mut Session) -> S2Fut<'f, Result<()>>
    where
        's: 'f,
        'a: 'f,
    {
        Box::pin(async move {
            let Session { io, store } = session;
            let interpretter = store.get_or_compose::<InterptetService>().clone();
            let tls_provider = store.get_ref::<TlsService>().cloned();
            let mut smtp = std::mem::take(store.get_or_compose::<SmtpSession>());
            if let Some(conn) = store.get_ref::<ConnectionInfo>() {
                if smtp.service_name.is_empty() {
                    smtp.service_name = conn.local_addr.clone()
                }
            }

            let mut io = BufReader::new(io);

            // fetch and apply commands
            loop {
                // write all pending responses
                while let Some(response) = smtp.pop_control() {
                    trace!("Processing driver control {:?}", response);
                    match response {
                        DriverControl::Response(bytes) => {
                            let writer = io.get_mut();
                            let write = writer.write_all(bytes.as_ref()).await;
                            writer.flush().await.and(write)?;
                        }
                        DriverControl::Shutdown => {
                            // CHECKME: why?
                            smtp.input.extend_from_slice(io.buffer());
                            // TODO: replace with close() after https://github.com/async-rs/async-std/issues/977
                            poll_fn(move |cx| Pin::new(io.get_mut()).poll_close(cx)).await?;
                            trace!("Shutdown completed");
                            return Ok(());
                        }
                        DriverControl::StartTls => {
                            use crate::io::tls::TlsProviderExt;
                            if let Some(ref tls) = tls_provider {
                                tls.upgrade_to_tls_in_place(io.get_mut(), String::default());
                            } else {
                                return Err(format!("no TLS").into());
                            }
                        }
                    }
                }
                let mut context = SmtpContext::new(store, &mut smtp);
                match interpretter.interpret(&mut context).await {
                    Ok(None) => {
                        // Action taken, but no input consumed (i.e. session setup / shut down)
                    }
                    Ok(Some(consumed)) => {
                        assert_ne!(consumed, 0, "If consumed is 0, infinite loop is likely.");
                        assert!(
                            consumed <= smtp.input.len(),
                            "The interpreter consumed more than a buffer? How?"
                        );
                        // TODO: handle buffer more efficiently, now allocating all the time
                        smtp.input = smtp.input.split_off(consumed);
                    }
                    Err(ParseError::Incomplete) => {
                        // TODO: take care of large chunks without LF
                        match io.read_until(b'\n', &mut smtp.input).await {
                            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                                warn!("session read timeout, prudence does this");
                                smtp.say_shutdown_timeout();
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => {
                                smtp.say_shutdown_processing_err("prudence does this".to_owned());
                            }
                            Err(e) => return Err(e.into()),
                            Ok(0) => {
                                if smtp.input.is_empty() {
                                    // client went silent, we're done!
                                    smtp.shutdown();
                                } else {
                                    error!(
                                        "Incomplete and finished: {:?}",
                                        String::from_utf8_lossy(smtp.input.as_slice())
                                    );
                                    // client did not finish the command and left.
                                    smtp.say_shutdown_processing_err("Incomplete command".into());
                                };
                            }
                            Ok(_) => { /* good, interpret again */ }
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Invalid command {:?} - {}",
                            String::from_utf8_lossy(smtp.input.as_slice()),
                            e
                        );

                        // remove one line from the buffer
                        let split = smtp
                            .input
                            .iter()
                            .position(|b| *b == b'\n')
                            .map(|p| p + 1)
                            .unwrap_or(smtp.input.len());
                        smtp.input = smtp.input.split_off(split);

                        if split == 0 {
                            warn!("Parsing failed on empty input, this will fail again, stopping the session");
                            smtp.say_shutdown_service_err()
                        } else {
                            smtp.say_invalid_syntax();
                        }
                    }
                };
            }
        })
    }
}
