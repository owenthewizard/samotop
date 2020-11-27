use crate::common::*;
use crate::smtp::{SmtpSessionCommand, SmtpState};

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct SmtpRset;

impl SmtpSessionCommand for SmtpRset {
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
        mail::Builder,
        smtp::{SmtpMail, SmtpPath},
    };
    use futures_await_test::async_test;

    #[async_test]
    async fn transaction_gets_reset() {
        let mut set = SmtpState::new(Builder::default());
        set.transaction.id = "someid".to_owned();
        set.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
        set.transaction.rcpts.push(SmtpPath::Null);
        set.transaction.extra_headers.insert_str(0, "feeeha");
        let sut = SmtpRset;
        let res = sut.apply(set).await;
        assert!(res.transaction.is_empty())
    }
}
