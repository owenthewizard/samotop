use super::{ESMTPCommand, Rfc5321};
use crate::{
    common::*,
    smtp::{ApplyCommand, SmtpNoop, SmtpSessionCommand, SmtpState},
};

impl SmtpSessionCommand for ESMTPCommand<SmtpNoop> {
    fn verb(&self) -> &str {
        "NOOP"
    }

    fn apply(&self, state: SmtpState) -> S2Fut<SmtpState> {
        Rfc5321::apply_cmd(&self.instruction, state)
    }
}

impl ApplyCommand<SmtpNoop> for Rfc5321 {
    fn apply_cmd(_cmd: &SmtpNoop, mut state: SmtpState) -> S2Fut<SmtpState> {
        state.say_ok();
        Box::pin(ready(state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::{Builder, Recipient},
        smtp::{SmtpMail, SmtpPath},
    };
    use futures_await_test::async_test;

    #[async_test]
    async fn transaction_gets_reset() {
        let mut set = SmtpState::new(Builder::default());
        set.transaction.id = "someid".to_owned();
        set.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
        set.transaction.rcpts.push(Recipient::null());
        set.transaction.extra_headers.insert_str(0, "feeeha");
        let sut = Rfc5321::new(SmtpNoop);
        let _res = sut.apply(set).await;
        // TODO: assert
    }
}
