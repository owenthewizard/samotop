use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use samotop_core::{
    mail::{Esmtp, MailSetup},
    smtp::{command::Timeout, Action, SmtpState},
    smtp::{Interpret, InterpretResult},
};
use smol_timeout::TimeoutExt;

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
        state: &'s mut samotop_core::smtp::SmtpState,
    ) -> samotop_core::common::S1Fut<'f, InterpretResult>
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
    fn setup(self, config: &mut samotop_core::mail::Configuration) {
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
            Some(res) => {
                state.session.last_command_at = Instant::now();
                res
            }
            None => {
                Esmtp.apply(Timeout, state).await;
                Err(todo!("timeout"))
            }
        }
    }
}
