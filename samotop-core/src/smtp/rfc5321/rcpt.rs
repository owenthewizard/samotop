use super::Esmtp;
use crate::{
    common::S2Fut,
    mail::{AddRecipientResult, MailGuardService, Recipient},
    smtp::{command::SmtpRcpt, Action, SmtpContext},
};

impl Action<SmtpRcpt> for Esmtp {
    fn apply<'a, 's, 'f>(&'a self, cmd: SmtpRcpt, state: &'s mut SmtpContext) -> S2Fut<'f, ()>
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
                .store
                .get_or_compose::<MailGuardService>()
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
    use crate::{
        smtp::{command::SmtpMail, SmtpPath, SmtpSession},
        store::Store,
    };

    #[test]
    fn recipient_is_added() {
        async_std::task::block_on(async move {
            
        let mut store = Store::default();
        let mut smtp = SmtpSession::default();
        let mut set = SmtpContext::new(&mut store, &mut smtp);

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
