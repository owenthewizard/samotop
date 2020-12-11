use super::{ESMTPCommand, Rfc5321};
use crate::{
    common::*,
    smtp::{ApplyCommand, SmtpSessionCommand, SmtpState, SmtpUnknownCommand},
};

impl SmtpSessionCommand for ESMTPCommand<SmtpUnknownCommand> {
    fn verb(&self) -> &str {
        self.instruction.verb.as_str()
    }

    fn apply(&self, state: SmtpState) -> S2Fut<SmtpState> {
        Rfc5321::apply_cmd(&self.instruction, state)
    }
}

impl ApplyCommand<SmtpUnknownCommand> for Rfc5321 {
    fn apply_cmd(_cmd: &SmtpUnknownCommand, mut state: SmtpState) -> S2Fut<SmtpState> {
        state.say_not_implemented();
        Box::pin(ready(state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::{Builder, Recipient},
        smtp::{CodecControl, SmtpMail, SmtpPath, SmtpState},
    };
    use futures_await_test::async_test;

    #[async_test]
    async fn response_is_not_implemented() {
        let mut set = SmtpState::new(Builder::default());
        set.transaction.id = "someid".to_owned();
        set.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
        set.transaction.rcpts.push(Recipient::null());
        set.transaction.extra_headers.insert_str(0, "feeeha");
        let sut = Rfc5321::command(SmtpUnknownCommand::new("HOOO".to_owned(), vec![]));
        let mut res = sut.apply(set).await;
        match res.writes.pop_front() {
            Some(CodecControl::Response(bytes)) if bytes.starts_with(b"502 ") => {}
            otherwise => panic!("Expected command not implemented, got {:?}", otherwise),
        }
    }
}
