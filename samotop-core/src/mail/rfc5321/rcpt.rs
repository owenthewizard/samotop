use super::{EsmtpCommand, Rfc5321};
use crate::{
    common::*,
    mail::{AddRecipientRequest, AddRecipientResult, Recipient},
    smtp::{ApplyCommand, SmtpRcpt, SmtpSessionCommand, SmtpState},
};

impl SmtpSessionCommand for EsmtpCommand<SmtpRcpt> {
    fn verb(&self) -> &str {
        "RCPT"
    }

    fn apply(&self, state: SmtpState) -> S1Fut<SmtpState> {
        Rfc5321::apply_cmd(&self.instruction, state)
    }
}

impl ApplyCommand<SmtpRcpt> for Rfc5321 {
    fn apply_cmd(cmd: &SmtpRcpt, mut state: SmtpState) -> S1Fut<SmtpState> {
        if state.transaction.mail.is_none() {
            state.say_command_sequence_fail();
            return Box::pin(ready(state));
        }
        let transaction = std::mem::take(&mut state.transaction);
        let request = AddRecipientRequest {
            transaction,
            rcpt: Recipient::new(cmd.0.clone()),
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
    async fn recipient_is_added() {
        let mut set = SmtpState::new(Builder::default());
        set.transaction.id = "someid".to_owned();
        set.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
        set.transaction.rcpts.push(Recipient::null());
        set.transaction.extra_headers.insert_str(0, "feeeha");
        let sut = Rfc5321::command(SmtpRcpt(SmtpPath::Postmaster, vec![]));
        let res = sut.apply(set).await;
        assert_eq!(res.transaction.rcpts.len(), 2);
    }
}
