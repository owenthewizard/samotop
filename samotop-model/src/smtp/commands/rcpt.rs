use crate::{
    common::*,
    mail::{AddRecipientRequest, AddRecipientResult},
    smtp::{SmtpPath, SmtpSessionCommand, SmtpState},
};

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct SmtpRcpt(pub SmtpPath, pub Vec<String>);

impl SmtpSessionCommand for SmtpRcpt {
    fn verb(&self) -> &str {
        "RCPT"
    }

    fn apply(&self, mut state: SmtpState) -> S2Fut<SmtpState> {
        if state.transaction.mail.is_none() {
            state.say_command_sequence_fail();
            return Box::pin(ready(state));
        }
        let transaction = std::mem::take(&mut state.transaction);
        let request = AddRecipientRequest {
            transaction,
            rcpt: self.0.clone(),
        };
        let fut = async move {
            match state.service.add_recipient(request).await {
                AddRecipientResult::Inconclusive(AddRecipientRequest {
                    mut transaction,
                    rcpt,
                }) => {
                    transaction.rcpts.push(rcpt);
                    state.say_ok();
                    state.transaction = transaction;
                }
                AddRecipientResult::TerminateSession(description) => {
                    state.say_shutdown_err(description);
                }
                AddRecipientResult::Failed(transaction, failure, description) => {
                    state.say_rcpt_failed(failure, description);
                    state.transaction = transaction;
                }
                AddRecipientResult::Accepted(transaction) => {
                    state.say_ok();
                    state.transaction = transaction;
                }
                AddRecipientResult::AcceptedWithNewPath(transaction, path) => {
                    state.say_ok_recipient_not_local(path);
                    state.transaction = transaction;
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
        let sut = SmtpRcpt(SmtpPath::Postmaster, vec![]);
        let res = sut.apply(set).await;
        assert_eq!(res.transaction.rcpts.len(), 2);
    }
}
