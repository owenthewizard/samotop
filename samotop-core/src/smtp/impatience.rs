use crate::{
    common::S1Fut,
    mail::{Configuration, MailSetup},
    smtp::{Interpret, InterpretResult},
    smtp::{Interpretter, SmtpState},
};
use async_std::prelude::FutureExt;
use std::time::{Duration, Instant};

/// Enforces the specified command timeout
#[derive(Debug)]
pub struct Impatience {
    timeout: Duration,
    inner: Box<dyn Interpret + Sync + Send>,
}

impl Interpret for Impatience {
    fn interpret<'a, 'i, 's, 'f>(
        &'a self,
        input: &'i [u8],
        state: &'s mut SmtpState,
    ) -> S1Fut<'f, InterpretResult>
    where
        'a: 'f,
        'i: 'f,
        's: 'f,
    {
        Box::pin(self.interpret_inner(input, state))
    }
}

#[derive(Debug, Clone)]
struct ImpatienceSetup {
    timeout: Duration,
}

impl MailSetup for ImpatienceSetup {
    fn setup(self, config: &mut Configuration) {
        let calls = std::mem::take(&mut config.interpret);
        config.interpret.insert(
            0,
            Box::new(Impatience::new(Interpretter::new(calls), self.timeout)),
        );
    }
}

impl Impatience {
    /// Gives a mail setup suitable for `.using(Impatience::timeout(Duration::from_seconds(10)))` calls
    pub fn timeout(timeout: Duration) -> impl MailSetup {
        ImpatienceSetup { timeout }
    }
    pub fn new(timeboxed: impl Interpret + Sync + Send + 'static, timeout: Duration) -> Self {
        Self {
            inner: Box::new(timeboxed),
            timeout,
        }
    }
    pub async fn interpret_inner(&self, input: &[u8], state: &mut SmtpState) -> InterpretResult {
        match self
            .inner
            .interpret(input, state)
            .timeout(self.timeout)
            .await
        {
            Ok(res) => {
                state.session.last_command_at = Instant::now();
                res
            }
            Err(_e) => {
                state.say_shutdown_timeout();
                Ok(None)
            }
        }
    }
}
