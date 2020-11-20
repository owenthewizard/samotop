use crate::{
    common::*,
    smtp::{SmtpSessionCommand, SmtpState},
};

#[derive(Default, Eq, PartialEq, Debug, Clone)]
pub struct SmtpUnknownCommand {
    verb: String,
    params: Vec<String>,
}

impl SmtpSessionCommand for SmtpUnknownCommand {
    fn verb(&self) -> &str {
        self.verb.as_str()
    }
    fn apply<'s, 'f, S>(self, mut state: S) -> S2Fut<'f, S>
    where
        S: SmtpState + 's,
        's: 'f,
    {
        state.say_not_implemented();
        Box::pin(ready(state))
    }
}

impl SmtpUnknownCommand {
    pub fn new(verb: String, params: Vec<String>) -> Self {
        Self { verb, params }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::Builder,
        smtp::{SmtpMail, SmtpPath, SmtpReply, SmtpStateBase, WriteControl},
    };
    use futures_await_test::async_test;

    #[async_test]
    async fn response_is_not_implemented() {
        let mut set = SmtpStateBase::new(Builder::default());
        set.transaction_mut().id = "someid".to_owned();
        set.transaction_mut().mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
        set.transaction_mut().rcpts.push(SmtpPath::Null);
        set.transaction_mut().extra_headers.insert_str(0, "feeeha");
        let sut = SmtpUnknownCommand::new("HOOO".to_owned(), vec![]);
        let mut res = sut.apply(set).await;
        assert_eq!(
            res.pop(),
            Some(WriteControl::Reply(SmtpReply::CommandNotImplementedFailure))
        );
    }
}
