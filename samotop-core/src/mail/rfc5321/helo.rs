use crate::{
    common::S1Fut,
    mail::Esmtp,
    smtp::{
        command::{SmtpHelo, SmtpUnknownCommand},
        Action, SmtpState,
    },
};

impl Action<SmtpHelo> for Esmtp {
    fn apply<'a, 's, 'f>(&'a self, cmd: SmtpHelo, state: &'s mut SmtpState) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
            match cmd.verb.to_ascii_uppercase().as_str() {
                "EHLO" => apply_helo(cmd, true, state),
                "HELO" => apply_helo(cmd, false, state),
                verb => {
                    Esmtp
                        .apply(
                            SmtpUnknownCommand::new(verb.to_owned(), vec![cmd.host.to_string()]),
                            state,
                        )
                        .await
                }
            }
        })
    }
}

/// Applies given helo to the state
/// It assumes it is the right HELO/EHLO/LHLO variant
pub fn apply_helo(helo: SmtpHelo, is_extended: bool, state: &mut SmtpState) {
    let local = state.session.service_name.to_owned();
    let remote = helo.host.to_string();

    state.reset_helo(helo.host.to_string());

    match is_extended {
        false => state.say_helo(local, remote),
        true => {
            let extensions = state.session.extensions.iter().map(String::from).collect();
            state.say_ehlo(local, extensions, remote)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::{Builder, Recipient},
        smtp::{command::SmtpMail, SmtpHost, SmtpPath},
    };

    #[test]
    fn transaction_gets_reset() {
        async_std::task::block_on(async move {
            let mut set = SmtpState::new(Builder::default().into_service());
            set.transaction.id = "someid".to_owned();
            set.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
            set.transaction.rcpts.push(Recipient::null());
            set.transaction.extra_headers.insert_str(0, "feeeha");

            Esmtp
                .apply(
                    SmtpHelo {
                        verb: "EHLO".to_string(),
                        host: SmtpHost::Domain("wex.xor.ro".to_owned()),
                    },
                    &mut set,
                )
                .await;
            assert!(set.transaction.is_empty());
        })
    }

    #[test]
    fn helo_is_set() {
        async_std::task::block_on(async move {
            let mut set = SmtpState::new(Builder::default().into_service());

            Esmtp
                .apply(
                    SmtpHelo {
                        verb: "EHLO".to_string(),
                        host: SmtpHost::Domain("wex.xor.ro".to_owned()),
                    },
                    &mut set,
                )
                .await;
            assert_eq!(set.session.peer_name, Some("wex.xor.ro".to_owned()));
        })
    }

    #[test]
    fn is_sync_and_send() {
        let mut set = SmtpState::new(Builder::default().into_service());
        let res = Esmtp.apply(
            SmtpHelo {
                verb: "EHLO".to_string(),
                host: SmtpHost::Domain("wex.xor.ro".to_owned()),
            },
            &mut set,
        );

        is_send(res);
    }

    fn is_send<T: Send>(_subj: T) {}
}
