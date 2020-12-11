use super::Rfc5321;
use crate::common::*;
use crate::smtp::{SmtpRset, SmtpSessionCommand, SmtpState};

impl SmtpSessionCommand for Rfc5321<SmtpRset> {
    fn verb(&self) -> &str {
        "RSET"
    }

    fn apply(&self, mut state: SmtpState) -> S2Fut<SmtpState> {
        state.reset();
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
        let sut = SmtpRset;
        let res = sut.apply(set).await;
        assert!(res.transaction.is_empty())
    }
}
