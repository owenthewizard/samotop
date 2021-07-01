use super::Esmtp;
use crate::smtp::{command::SmtpUnknownCommand, Action, SmtpState};

#[async_trait::async_trait]
impl Action<SmtpUnknownCommand> for Esmtp {
    async fn apply(&self, _cmd: SmtpUnknownCommand, state: &mut SmtpState) {
        state.say_not_implemented();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::{Builder, Recipient},
        smtp::{command::SmtpMail, DriverControl, SmtpPath, SmtpState},
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
            .apply(SmtpUnknownCommand::new("HOOO".to_owned(), vec![]), &mut set)
            .await;
        match set.writes.pop_front() {
            Some(DriverControl::Response(bytes)) if bytes.starts_with(b"502 ") => {}
            otherwise => panic!("Expected command not implemented, got {:?}", otherwise),
        }
    }
}
