use crate::{
    common::*,
    smtp::{SmtpSessionCommand, SmtpState},
};

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct SmtpNoop;

impl SmtpSessionCommand for SmtpNoop {
    fn verb(&self) -> &str {
        "NOOP"
    }
    fn apply<'s, 'f, S>(self, mut state: S) -> S2Fut<'f, S>
    where
        S: SmtpState + 's,
        's: 'f,
    {
        state.say_ok();
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
        let sut = SmtpNoop;
        let _res = sut.apply(set).await;
        // TODO: assert
    }
}
