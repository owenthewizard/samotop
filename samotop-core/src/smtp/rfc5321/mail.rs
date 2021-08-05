use crate::{
    common::{Identify, S1Fut},
    mail::{MailGuard, StartMailResult},
    smtp::{command::SmtpMail, Action, Esmtp, SmtpContext},
};

impl Action<SmtpMail> for Esmtp {
    fn apply<'a, 's, 'f>(&'a self, cmd: SmtpMail, state: &'s mut SmtpContext) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
            if state.session.peer_name.is_none() {
                state.session.say_command_sequence_fail();
                return;
            }
            state.session.reset();
            state.session.transaction.mail = Some(cmd);

            use StartMailResult as R;
            match state.service().start_mail(&mut state.session).await {
                R::Failed(failure, description) => {
                    state.session.say_mail_failed(failure, description);
                }
                R::Accepted => {
                    if state.session.transaction.id.is_empty() {
                        let id = format!("{}@{}", Identify::now(), state.session.service_name);
                        warn!(
                            "Mail transaction ID is empty. Will use time based ID {}",
                            id
                        );
                        state.session.transaction.id = id;
                    }
                    state.session.say_ok_info(format!(
                        "Ok! Transaction {} started.",
                        state.session.transaction.id
                    ));
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::Recipient,
        smtp::{command::SmtpMail, DriverControl, Esmtp, SmtpPath},
    };

    #[test]
    fn transaction_gets_reset() {
        async_std::task::block_on(async move {
            let mut set = SmtpContext::default();
            set.session.peer_name = Some("xx.io".to_owned());
            set.session.transaction.id = "someid".to_owned();
            set.session.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
            set.session.transaction.rcpts.push(Recipient::null());
            set.session
                .transaction
                .extra_headers
                .insert_str(0, "feeeha");

            Esmtp
                .apply(SmtpMail::Mail(SmtpPath::Postmaster, vec![]), &mut set)
                .await;
            match set.session.pop_control() {
                Some(DriverControl::Response(bytes)) if bytes.starts_with(b"250 ") => {}
                otherwise => panic!("Expected OK, got {:?}", otherwise),
            }
            assert_ne!(set.session.transaction.id, "someid");
            assert!(set.session.transaction.rcpts.is_empty());
            assert!(set.session.transaction.extra_headers.is_empty());
        })
    }

    #[test]
    fn mail_is_set() {
        async_std::task::block_on(async move {
            let mut set = SmtpContext::default();
            set.session.peer_name = Some("xx.io".to_owned());

            Esmtp
                .apply(SmtpMail::Mail(SmtpPath::Postmaster, vec![]), &mut set)
                .await;
            match set.session.pop_control() {
                Some(DriverControl::Response(bytes)) if bytes.starts_with(b"250 ") => {}
                otherwise => panic!("Expected OK, got {:?}", otherwise),
            }
            assert_eq!(
                set.session.transaction.mail,
                Some(SmtpMail::Mail(SmtpPath::Postmaster, vec![]))
            );
        })
    }

    #[test]
    fn command_sequence_is_enforced() {
        async_std::task::block_on(async move {
            // MAIL command requires HELO/EHLO
            let mut set = SmtpContext::default();

            Esmtp
                .apply(SmtpMail::Mail(SmtpPath::Postmaster, vec![]), &mut set)
                .await;
            match set.session.pop_control() {
                Some(DriverControl::Response(bytes)) if bytes.starts_with(b"503 ") => {}
                otherwise => panic!("Expected command sequence failure, got {:?}", otherwise),
            }
            assert_eq!(set.session.transaction.mail, None);
        })
    }
}
