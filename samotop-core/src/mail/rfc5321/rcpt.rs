use super::Esmtp;
use crate::{
    common::S1Fut,
    mail::{AddRecipientRequest, AddRecipientResult, Recipient},
    smtp::{command::SmtpRcpt, Action, SmtpState},
};

impl Action<SmtpRcpt> for Esmtp {
    fn apply<'a, 's, 'f>(&'a self, cmd: SmtpRcpt, state: &'s mut SmtpState) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
            if state.transaction.mail.is_none() {
                state.say_command_sequence_fail();
                return;
            }
            let transaction = std::mem::take(&mut state.transaction);
            let request = AddRecipientRequest {
                transaction,
                rcpt: Recipient::new(cmd.0.clone()),
            };

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
                    state.say_shutdown_service_err(description);
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
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::Builder,
        smtp::{command::SmtpMail, SmtpPath},
    };

    #[test]
    fn recipient_is_added() {
        async_std::task::block_on(async move {
            let mut set = SmtpState::new(Builder::default().into_service());
            set.transaction.id = "someid".to_owned();
            set.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
            set.transaction.rcpts.push(Recipient::null());
            set.transaction.extra_headers.insert_str(0, "feeeha");

            Esmtp
                .apply(SmtpRcpt(SmtpPath::Postmaster, vec![]), &mut set)
                .await;
            assert_eq!(set.transaction.rcpts.len(), 2);
        })
    }
}
