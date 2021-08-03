use super::Esmtp;
use crate::{
    common::S1Fut,
    mail::{AddRecipientResult, MailGuard, Recipient},
    smtp::{command::SmtpRcpt, Action, SmtpContext},
};

impl Action<SmtpRcpt> for Esmtp {
    fn apply<'a, 's, 'f>(&'a self, cmd: SmtpRcpt, state: &'s mut SmtpContext) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
            if state.session.transaction.mail.is_none() {
                state.session.say_command_sequence_fail();
                return;
            }
            let rcpt = Recipient::new(cmd.0.clone());

            match state
                .service()
                .add_recipient(&mut state.session, rcpt)
                .await
            {
                AddRecipientResult::Inconclusive(rcpt) => {
                    state.session.transaction.rcpts.push(rcpt);
                    state.session.say_ok();
                }
                AddRecipientResult::Failed(failure, description) => {
                    state.session.say_rcpt_failed(failure, description);
                }
                AddRecipientResult::Accepted => {
                    state.session.say_ok();
                }
                AddRecipientResult::AcceptedWithNewPath(path) => {
                    state.session.say_ok_recipient_not_local(path);
                }
            };
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::smtp::{command::SmtpMail, SmtpPath};

    #[test]
    fn recipient_is_added() {
        async_std::task::block_on(async move {
            let mut set = SmtpContext::default();
            set.session.transaction.id = "someid".to_owned();
            set.session.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
            set.session.transaction.rcpts.push(Recipient::null());
            set.session
                .transaction
                .extra_headers
                .insert_str(0, "feeeha");

            Esmtp
                .apply(SmtpRcpt(SmtpPath::Postmaster, vec![]), &mut set)
                .await;
            assert_eq!(set.session.transaction.rcpts.len(), 2);
        })
    }
}
