use super::Esmtp;
use crate::smtp::{command::SmtpInvalidCommand, Action, SmtpState};

#[async_trait::async_trait]
impl Action<SmtpInvalidCommand> for Esmtp {
    async fn apply(&self, cmd: SmtpInvalidCommand, state: &mut SmtpState) {
        state.say_invalid_syntax();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::{Builder, Recipient},
        smtp::{command::SmtpMail, DriverControl, SmtpPath},
    };
    use futures_await_test::async_test;

    #[async_test]
    async fn response_is_not_implemented() {
        let mut set = SmtpState::new(Builder::default().into_service());
        set.transaction.id = "someid".to_owned();
        set.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
        set.transaction.rcpts.push(Recipient::null());
        set.transaction.extra_headers.insert_str(0, "feeeha");

        Esmtp
            .apply(SmtpInvalidCommand::new(b"HOOO".to_vec()), &mut set)
            .await;
        match set.writes.pop_front() {
            Some(DriverControl::Response(bytes)) if bytes.starts_with(b"500 ") => {}
            otherwise => panic!("Expected syntax failure, got {:?}", otherwise),
        }
    }
}
