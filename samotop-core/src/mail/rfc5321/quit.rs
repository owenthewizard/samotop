use super::{EsmtpCommand, Rfc5321};
use crate::common::*;
use crate::smtp::{ApplyCommand, SmtpQuit, SmtpSessionCommand, SmtpState};

impl SmtpSessionCommand for EsmtpCommand<SmtpQuit> {
    fn verb(&self) -> &str {
        "QUIT"
    }

    fn apply(&self, state: SmtpState) -> S2Fut<SmtpState> {
        Rfc5321::apply_cmd(&self.instruction, state)
    }
}

impl ApplyCommand<SmtpQuit> for Rfc5321 {
    fn apply_cmd(_cmd: &SmtpQuit, mut state: SmtpState) -> S2Fut<SmtpState> {
        let name = state.session.service_name.clone();
        state.reset();
        state.say_shutdown_ok(name);
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
        let sut = Rfc5321::command(SmtpQuit);
        let res = sut.apply(set).await;
        assert!(res.transaction.is_empty())
    }
}
