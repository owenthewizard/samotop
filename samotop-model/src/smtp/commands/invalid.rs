use crate::{
    common::*,
    smtp::{SmtpSessionCommand, SmtpState},
};

#[derive(Default, Eq, PartialEq, Debug, Clone)]
pub struct SmtpInvalidCommand {
    line: Vec<u8>,
}

impl SmtpSessionCommand for SmtpInvalidCommand {
    fn verb(&self) -> &str {
        ""
    }

    fn apply(&self, mut state: SmtpState) -> S2Fut<SmtpState> {
        state.say_invalid_syntax();
        Box::pin(ready(state))
    }
}

impl SmtpInvalidCommand {
    pub fn new(line: Vec<u8>) -> Self {
        Self { line }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::Builder,
        smtp::{CodecControl, SmtpMail, SmtpPath},
    };
    use futures_await_test::async_test;

    #[async_test]
    async fn response_is_not_implemented() {
        let mut set = SmtpState::new(Builder::default());
        set.transaction.id = "someid".to_owned();
        set.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
        set.transaction.rcpts.push(SmtpPath::Null);
        set.transaction.extra_headers.insert_str(0, "feeeha");
        let sut = SmtpInvalidCommand::new(b"HOOO".to_vec());
        let mut res = sut.apply(set).await;
        match res.writes.pop_front() {
            Some(CodecControl::Response(bytes)) if bytes.starts_with(b"500 ") => {}
            otherwise => panic!("Expected syntax failure, got {:?}", otherwise),
        }
    }
}
