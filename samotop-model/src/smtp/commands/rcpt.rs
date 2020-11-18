use crate::{
    common::*,
    mail::{AddRecipientRequest, AddRecipientResult},
    smtp::{SmtpPath, SmtpSessionCommand, SmtpState},
};

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct SmtpRcpt(SmtpPath);

impl From<SmtpPath> for SmtpRcpt {
    fn from(from: SmtpPath) -> Self {
        Self(from)
    }
}

impl SmtpSessionCommand for SmtpRcpt {
    fn verb(&self) -> &str {
        "RCPT"
    }
    fn apply<'s, 'f, S>(self, mut state: S) -> S2Fut<'f, S>
    where
        S: SmtpState + 's,
        's: 'f,
    {
        if state.transaction().mail.is_none() {
            state.say_command_sequence_fail();
            return Box::pin(ready(state));
        }
        let transaction = std::mem::take(state.transaction_mut());
        let request = AddRecipientRequest {
            transaction,
            rcpt: self.0,
        };
        let fut = async move {
            match state.service().add_recipient(request).await {
                AddRecipientResult::Inconclusive(AddRecipientRequest {
                    mut transaction,
                    rcpt,
                }) => {
                    transaction.rcpts.push(rcpt);
                    *state.transaction_mut() = transaction;
                }
                AddRecipientResult::TerminateSession(description) => {
                    state.say_shutdown_err(description);
                }
                AddRecipientResult::Failed(transaction, failure, description) => {
                    state.say_rcpt_failed(failure, description);
                    *state.transaction_mut() = transaction;
                }
                AddRecipientResult::Accepted(transaction) => {
                    state.say_ok();
                    *state.transaction_mut() = transaction;
                }
                AddRecipientResult::AcceptedWithNewPath(transaction, path) => {
                    state.say_ok_recipient_not_local(path);
                    *state.transaction_mut() = transaction;
                }
            };
            state
        };

        Box::pin(fut)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::smtp::{SmtpMail, SmtpPath, SmtpStateBase};
    use futures_await_test::async_test;

    #[async_test]
    async fn transaction_gets_reset() {
        let mut set = SmtpStateBase::default();
        set.transaction_mut().id = "someid".to_owned();
        set.transaction_mut().mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
        set.transaction_mut().rcpts.push(SmtpPath::Null);
        set.transaction_mut().extra_headers.insert_str(0, "feeeha");
        let sut = SmtpRcpt(SmtpPath::Postmaster);
        let res = sut.apply(set).await;
        assert_eq!(res.transaction().rcpts.len(), 2);
    }
}
