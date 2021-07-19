use super::Esmtp;
use crate::{
    common::S1Fut,
    mail::{StartMailFailure, StartMailResult, Transaction},
    smtp::{command::SmtpMail, Action, SmtpState},
};

impl Action<SmtpMail> for Esmtp {
    fn apply<'a, 's, 'f>(&'a self, cmd: SmtpMail, state: &'s mut SmtpState) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
            if state.session.peer_name.is_none() {
                state.say_command_sequence_fail();
                return;
            }
            state.reset();

            let transaction = Transaction {
                mail: Some(cmd.clone()),
                ..Transaction::default()
            };

            use StartMailResult as R;
            match state.service.start_mail(&state.session, transaction).await {
                R::Failed(StartMailFailure::TerminateSession, description) => {
                    state.say_shutdown_service_err(description);
                }
                R::Failed(failure, description) => {
                    state.say_mail_failed(failure, description);
                }
                R::Accepted(mut transaction) => {
                    if transaction.id.is_empty() {
                        fn nunnumber(input: char) -> bool {
                            !input.is_ascii_digit()
                        }
                        // for the lack of better unique string without extra dependencies
                        let id = format!("{:?}", std::time::Instant::now()).replace(nunnumber, "");
                        warn!(
                            "Mail transaction ID is empty. Will use time based ID {}",
                            id
                        );
                        transaction.id = id;
                    }
                    state.say_ok_info(format!("Ok! Transaction {} started.", transaction.id));
                    state.transaction = transaction;
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::{Builder, Esmtp, Recipient},
        smtp::{command::SmtpMail, DriverControl, SmtpPath},
    };

    #[test]
    fn transaction_gets_reset() {
        async_std::task::block_on(async move {
            let mut set = SmtpState::new(Builder::default().build());
            set.session.peer_name = Some("xx.io".to_owned());
            set.transaction.id = "someid".to_owned();
            set.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
            set.transaction.rcpts.push(Recipient::null());
            set.transaction.extra_headers.insert_str(0, "feeeha");

            Esmtp
                .apply(SmtpMail::Mail(SmtpPath::Postmaster, vec![]), &mut set)
                .await;
            match set.writes.pop_front() {
                Some(DriverControl::Response(bytes)) if bytes.starts_with(b"250 ") => {}
                otherwise => panic!("Expected OK, got {:?}", otherwise),
            }
            assert_ne!(set.transaction.id, "someid");
            assert!(set.transaction.rcpts.is_empty());
            assert!(set.transaction.extra_headers.is_empty());
        })
    }

    #[test]
    fn mail_is_set() {
        async_std::task::block_on(async move {
            let mut set = SmtpState::new(Builder::default().build());
            set.session.peer_name = Some("xx.io".to_owned());

            Esmtp
                .apply(SmtpMail::Mail(SmtpPath::Postmaster, vec![]), &mut set)
                .await;
            match set.writes.pop_front() {
                Some(DriverControl::Response(bytes)) if bytes.starts_with(b"250 ") => {}
                otherwise => panic!("Expected OK, got {:?}", otherwise),
            }
            assert_eq!(
                set.transaction.mail,
                Some(SmtpMail::Mail(SmtpPath::Postmaster, vec![]))
            );
        })
    }

    #[test]
    fn command_sequence_is_enforced() {
        async_std::task::block_on(async move {
            // MAIL command requires HELO/EHLO
            let mut set = SmtpState::new(Builder::default().build());

            Esmtp
                .apply(SmtpMail::Mail(SmtpPath::Postmaster, vec![]), &mut set)
                .await;
            match set.writes.pop_front() {
                Some(DriverControl::Response(bytes)) if bytes.starts_with(b"503 ") => {}
                otherwise => panic!("Expected command sequence failure, got {:?}", otherwise),
            }
            assert_eq!(set.transaction.mail, None);
        })
    }
}
