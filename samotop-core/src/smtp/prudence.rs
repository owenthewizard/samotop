use crate::builder::{ServerContext, Setup};
use crate::common::*;
use crate::io::{ConnectionInfo, Handler, HandlerService, Io};
use crate::smtp::{Interpret, InterpretResult, ParseError, SmtpContext};
use crate::store::{Component, SingleComponent};
use smol_timeout::TimeoutExt;
use std::time::{Duration, Instant};

use super::InterptetService;

/// Prevent bad SMTP behavior
#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Prudence {
    /// Monitor bad behavior of clients not waiting for a banner given time
    wait_for_banner_delay: Option<Duration>,
    /// Maximum read time
    read_timeout: Option<Duration>,
}

impl Prudence {
    /// Shut the session down if the client sends commands before the delayed banner
    pub fn with_banner_delay(mut self, delay: Duration) -> Self {
        self.wait_for_banner_delay = Some(delay);
        self
    }
    /// Shut the session down if the client takes too long to send a command
    pub fn with_read_timeout(mut self, timeout: Duration) -> Self {
        self.read_timeout = Some(timeout);
        self
    }
}

impl Setup for Prudence {
    fn setup(&self, builder: &mut ServerContext) {
        let others = builder.store.get_or_compose::<HandlerService>().clone();
        builder
            .store
            .add::<HandlerService>(Arc::new(PrudentHandler {
                config: self.clone(),
                others,
            }));
    }
}

#[derive(Debug)]
struct PrudentHandler {
    config: Prudence,
    others: Arc<dyn Handler + Sync + Send>,
}

impl Handler for PrudentHandler {
    // fn setup_session<'a, 'i, 's, 'f>(
    //     &'a self,
    //     io: &'i mut Box<dyn MayBeTls>,
    //     state: &'s mut SmtpContext,
    // ) -> crate::common::S1Fut<'f, ()>
    // where
    //     'a: 'f,
    //     'i: 'f,
    //     's: 'f,
    // {
    //     Box::pin(async move {
    //         if let Some(timeout) = self.config.read_timeout {
    //             if let Some(interpretter) = state.store.get_or_compose::<InterptetService>() {
    //                 let prin = PrudentInterpretter {
    //                     inner: Box::new(interpretter),
    //                     timeout,
    //                 };
    //                 state.store.set::<InterptetService>(Arc::new(prin));
    //             }
    //         }

    //         if let Some(delay) = self.config.wait_for_banner_delay {
    //             let mut buf = [0u8; 425];
    //             use async_std::io::ReadExt;
    //             match io.read(&mut buf[..]).timeout(delay).await {
    //                 Some(Ok(0)) => {
    //                     // this just looks like the client gave up and left
    //                     warn!(
    //                         "{:?} touch and go!",
    //                         state.store.get_ref::<ConnectionInfo>()
    //                     );
    //                 }
    //                 Some(Ok(len)) => {
    //                     state.session.input.extend_from_slice(&buf[0..len]);
    //                     state.session.say_shutdown_processing_err(
    //                         "Client sent commands before banner".into(),
    //                     );
    //                 }
    //                 Some(Err(e)) => {
    //                     state
    //                         .session
    //                         .say_shutdown_processing_err(format!("IO read failed {}", e));
    //                 }
    //                 None => {
    //                     // timeout is correct behavior, well done!
    //                 }
    //             }
    //         }

    //         *io = Box::new(PrudentIo::new(
    //             self.config.read_timeout,
    //             std::mem::replace(io, Box::new(Dummy)),
    //         ));

    //         self.others.setup_session(io, state).await;
    //     })
    // }

    fn handle<'s, 'a, 'f>(
        &'s self,
        session: &'a mut crate::server::Session,
    ) -> S2Fut<'f, Result<()>>
    where
        's: 'f,
        'a: 'f,
    {
        Box::pin(async move {
            if let Some(timeout) = self.config.read_timeout {
                let others = session.store.get_or_compose::<InterptetService>().clone();
                let prin = PrudentInterpretter { others, timeout };
                // wrap interpretters
                session.store.set::<InterptetService>(Arc::new(prin));
            }

            if let Some(delay) = self.config.wait_for_banner_delay {
                let mut buf = [0u8; 425];
                use async_std::io::ReadExt;
                use async_std::io::WriteExt;
                match session.io.read(&mut buf[..]).timeout(delay).await {
                    Some(Ok(0)) => {
                        // this just looks like the client gave up and left
                        warn!(
                            "{:?} touch and go!",
                            session.store.get_ref::<ConnectionInfo>()
                        );
                    }
                    Some(Ok(_len)) => {
                        session
                            .io
                            .write_all("451 Requested action aborted\r\n".as_bytes())
                            .await?;
                        return Err("Client sent commands before banner".into());
                    }
                    Some(Err(e)) => {
                        session
                            .io
                            .write_all("451 Requested action aborted\r\n".as_bytes())
                            .await?;
                        return Err(e.into());
                    }
                    None => {
                        // timeout is correct behavior, well done!
                    }
                }
            }

            session.io = Box::new(PrudentIo::new(
                self.config.read_timeout,
                std::mem::replace(&mut session.io, Box::new(FallBack)),
            ));

            self.others.handle(session).await
        })
    }
}

#[derive(Debug)]
struct PrudentState {
    pub last_command_at: Instant,
}
impl Component for PrudentState {
    type Target = PrudentState;
}
impl SingleComponent for PrudentState {}

struct PrudentIo {
    expired: Pin<Box<dyn Future<Output = ()> + Sync + Send>>,
    timeout: Option<Duration>,
    io: Box<dyn Io>,
}

impl PrudentIo {
    pub fn new<IO: Io + 'static>(timeout: Option<Duration>, io: IO) -> Self {
        PrudentIo {
            expired: Box::pin(Self::expire(timeout)),
            timeout,
            io: Box::new(io),
        }
    }
    async fn expire(timeout: Option<Duration>) {
        if let Some(timeout) = timeout {
            pending::<()>().timeout(timeout).await;
        } else {
            pending::<()>().await;
        }
    }
}

impl io::Read for PrudentIo {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        if let Poll::Ready(()) = self.expired.as_mut().poll(cx) {
            return Poll::Ready(Err(io::ErrorKind::TimedOut.into()));
        }

        let res = Pin::new(&mut self.io).poll_read(cx, buf);

        if let Poll::Ready(Ok(_)) = res {
            self.expired = Box::pin(Self::expire(self.timeout))
        }

        res
    }
}
impl io::Write for PrudentIo {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.io).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.io).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.io).poll_close(cx)
    }
}

/// Enforces the specified command timeout
#[derive(Debug)]
struct PrudentInterpretter {
    others: Arc<dyn Interpret + Sync + Send>,
    timeout: Duration,
}

impl Interpret for PrudentInterpretter {
    fn interpret<'a, 's, 'f>(&'a self, state: &'s mut SmtpContext) -> S2Fut<'f, InterpretResult>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(self.interpret_inner(state))
    }
}

impl PrudentInterpretter {
    pub async fn interpret_inner<'a, 's>(
        &'a self,
        state: &'s mut SmtpContext<'_>,
    ) -> InterpretResult {
        let res = self.others.interpret(state).await;

        let mystate = state
            .store
            .get_or_insert::<PrudentState, _>(|| PrudentState {
                last_command_at: Instant::now(),
            });

        match res {
            Ok(Some(consumed)) if consumed != 0 => {
                mystate.last_command_at = std::time::Instant::now();
            }
            Err(ParseError::Incomplete) => {
                if Instant::now().saturating_duration_since(mystate.last_command_at) > self.timeout
                {
                    state.session.say_shutdown_timeout();
                    return Ok(None);
                }
            }
            _ => {}
        }

        res
    }
}
