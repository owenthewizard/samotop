use crate::{
    common::S1Fut,
    mail::{Configuration, Esmtp, MailSetup},
    smtp::{command::Timeout, Action, SmtpState},
    smtp::{Interpret, InterpretResult},
};
use async_std::prelude::FutureExt;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

/// Applies the specified command timeout
#[derive(Debug)]
pub struct Impatient {
    timeout: Duration,
    inner: Box<dyn Interpret + Sync + Send>,
}

impl Interpret for Impatient {
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
struct ImpatientSetup {
    timeout: Duration,
}

impl MailSetup for ImpatientSetup {
    fn setup(self, config: &mut Configuration) {
        config.interpretter = Box::new(Arc::new(Impatient::new(
            config.interpretter.clone(),
            self.timeout,
        )))
    }
}

impl Impatient {
    /// Gives a mail setup suitable for `.using(Impatient::after(Duration::from_seconds(10)))` calls
    pub fn after(timeout: Duration) -> impl MailSetup {
        ImpatientSetup { timeout }
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
                Esmtp.apply(Timeout, state).await;
                Ok(0)
            }
        }
    }
}
