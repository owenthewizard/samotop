use super::Esmtp;
use crate::smtp::{command::SmtpQuit, Action, SmtpState};

#[async_trait::async_trait]
impl Action<SmtpQuit> for Esmtp {
    async fn apply(&self, _cmd: SmtpQuit, state: &mut SmtpState) {
        let name = state.session.service_name.clone();
        state.reset();
        state.say_shutdown_ok(name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::{Builder, Recipient},
        smtp::{command::SmtpMail, SmtpPath},
    };
    use futures_await_test::async_test;

    #[async_test]
    async fn transaction_gets_reset() {
        let mut set = SmtpState::new(Builder::default().into_service());
        set.transaction.id = "someid".to_owned();
        set.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
        set.transaction.rcpts.push(Recipient::null());
        set.transaction.extra_headers.insert_str(0, "feeeha");

        Esmtp.apply(SmtpQuit, &mut set).await;
        assert!(set.transaction.is_empty())
    }
}
