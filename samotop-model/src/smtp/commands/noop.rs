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

    fn apply<'a>(&'a self, mut state: SmtpState) -> S2Fut<'a, SmtpState> {
        state.say_ok();
        Box::pin(ready(state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::Builder,
        smtp::{SmtpMail, SmtpPath},
    };
    use futures_await_test::async_test;

    #[async_test]
    async fn transaction_gets_reset() {
        let mut set = SmtpState::new(Builder::default());
        set.transaction.id = "someid".to_owned();
        set.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
        set.transaction.rcpts.push(SmtpPath::Null);
        set.transaction.extra_headers.insert_str(0, "feeeha");
        let sut = SmtpNoop;
        let _res = sut.apply(set).await;
        // TODO: assert
    }
}
