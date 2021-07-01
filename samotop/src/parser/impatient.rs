use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use samotop_core::{
    mail::MailSetup,
    parser::{Interpret, InterpretResult},
    smtp::{SmtpSessionCommand, Timeout},
};
use smol_timeout::TimeoutExt;

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
        todo!()
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
    pub async fn interpret_inner(
        &self,
        input: &[u8],
        state: &mut samotop_core::smtp::SmtpState,
    ) -> InterpretResult {
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
                let s = std::mem::take(state);
                *state = Timeout::new().apply(s).await;
                Err(todo!("timeout"))
            }
        }
    }
}
