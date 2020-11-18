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
    fn apply<'s, 'f, S>(self, mut state: S) -> S2Fut<'f, S>
    where
        S: SmtpState + 's,
        's: 'f,
    {
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
    use crate::smtp::{SmtpMail, SmtpPath, SmtpReply, SmtpStateBase, WriteControl};
    use futures_await_test::async_test;

    #[async_test]
    async fn response_is_not_implemented() {
        let mut set = SmtpStateBase::default();
        set.transaction_mut().id = "someid".to_owned();
        set.transaction_mut().mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
        set.transaction_mut().rcpts.push(SmtpPath::Null);
        set.transaction_mut().extra_headers.insert_str(0, "feeeha");
        let sut = SmtpInvalidCommand::new(b"HOOO".to_vec());
        let mut res = sut.apply(set).await;
        assert_eq!(
            res.pop(),
            Some(WriteControl::Reply(SmtpReply::CommandSyntaxFailure))
        );
    }
}
