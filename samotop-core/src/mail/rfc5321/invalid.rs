use super::Rfc5321;
use crate::{
    common::*,
    smtp::{SmtpInvalidCommand, SmtpSessionCommand, SmtpState},
};

impl SmtpSessionCommand for Rfc5321<SmtpInvalidCommand> {
    fn verb(&self) -> &str {
        ""
    }

    fn apply(&self, mut state: SmtpState) -> S2Fut<SmtpState> {
        state.say_invalid_syntax();
        Box::pin(ready(state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::{Builder, Recipient},
        smtp::{CodecControl, SmtpMail, SmtpPath},
    };
    use futures_await_test::async_test;

    #[async_test]
    async fn response_is_not_implemented() {
        let mut set = SmtpState::new(Builder::default());
        set.transaction.id = "someid".to_owned();
        set.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
        set.transaction.rcpts.push(Recipient::null());
        set.transaction.extra_headers.insert_str(0, "feeeha");
        let sut = SmtpInvalidCommand::new(b"HOOO".to_vec());
        let mut res = sut.apply(set).await;
        match res.writes.pop_front() {
            Some(CodecControl::Response(bytes)) if bytes.starts_with(b"500 ") => {}
            otherwise => panic!("Expected syntax failure, got {:?}", otherwise),
        }
    }
}
