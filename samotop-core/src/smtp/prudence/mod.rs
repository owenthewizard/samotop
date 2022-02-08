mod timelio;
mod waitio;

use self::timelio::ReadTimeoutIo;
use self::waitio::WaitIo;
use super::InterptetService;
use crate::common::*;
use crate::config::{Component, SingleComponent};
use crate::config::{ServerContext, Setup};
use crate::io::{Handler, HandlerService, Session};
use crate::smtp::{Interpret, InterpretResult, ParseError, SmtpContext};
use async_std::task::sleep;
use std::time::{Duration, Instant};

/// Prevent bad SMTP behavior
///
/// read timeout is applied twice
///  - first on IO level on individual reads, but IO has no concept of commands.
///  - second on interpretter level, this prevents the client from slowly drip feeding.
///
/// wait for banner ensures that the client did not send commands before seeing the banner
///  - this is a violation of RFCs indicating abusers.
///
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
    fn handle<'s, 'a, 'f>(&'s self, session: &'a mut Session) -> S2Fut<'f, Result<()>>
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

            session.io = Box::new(WaitIo::new(
                self.config.wait_for_banner_delay.unwrap_or_default(),
                ReadTimeoutIo::new(
                    self.config.read_timeout.unwrap_or_default(),
                    std::mem::replace(&mut session.io, Box::new(FallBack)),
                ),
            ));

            self.others.handle(session).await
        })
    }
}

pub(super) fn delay(
    t: Duration,
) -> Option<Pin<Box<dyn Future<Output = ()> + Sync + Send + 'static>>> {
    if t.is_zero() {
        return None;
    }
    Some(Box::pin(sleep(t)))
}

#[derive(Debug)]
struct PrudentState {
    pub last_command_at: Instant,
}
impl Component for PrudentState {
    type Target = PrudentState;
}
impl SingleComponent for PrudentState {}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn __() {}
}
