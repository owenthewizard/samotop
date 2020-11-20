use crate::common::*;
use crate::smtp::{SmtpSessionCommand, SmtpState};

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct SmtpQuit;

impl SmtpSessionCommand for SmtpQuit {
    fn verb(&self) -> &str {
        "QUIT"
    }
    fn apply<'s, 'f, S>(self, mut state: S) -> S2Fut<'f, S>
    where
        S: SmtpState + 's,
        's: 'f,
    {
        let name = state.session().service_name.clone();
        state.reset();
        state.say_shutdown_ok(name);
        Box::pin(ready(state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::Builder,
        smtp::{SmtpMail, SmtpPath, SmtpStateBase},
    };
    use futures_await_test::async_test;

    #[async_test]
    async fn transaction_gets_reset() {
        let mut set = SmtpStateBase::new(Builder::default());
        set.transaction_mut().id = "someid".to_owned();
        set.transaction_mut().mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
        set.transaction_mut().rcpts.push(SmtpPath::Null);
        set.transaction_mut().extra_headers.insert_str(0, "feeeha");
        let sut = SmtpQuit;
        let res = sut.apply(set).await;
        assert!(res.transaction().is_empty())
    }
}
