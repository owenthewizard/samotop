use crate::{
    common::S2Fut,
    smtp::{
        command::{SmtpHelo, SmtpUnknownCommand},
        Action, Esmtp, SmtpContext,
    },
};

impl Action<SmtpHelo> for Esmtp {
    fn apply<'a, 's, 'f>(&'a self, cmd: SmtpHelo, state: &'s mut SmtpContext) -> S2Fut<'f, ()>
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
pub fn apply_helo(helo: SmtpHelo, is_extended: bool, state: &mut SmtpContext) {
    state.session.reset_helo(helo.host.to_string());

    match is_extended {
        false => state.session.say_helo(),
        true => state.session.say_ehlo(),
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::Recipient,
        smtp::{command::SmtpMail, SmtpHost, SmtpPath, SmtpSession},
        store::Store,
    };

    #[test]
    fn transaction_gets_reset() {
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
                .apply(
                    SmtpHelo {
                        verb: "EHLO".to_string(),
                        host: SmtpHost::Domain("wex.xor.ro".to_owned()),
                    },
                    &mut set,
                )
                .await;
            assert!(set.session.transaction.is_empty());
        })
    }

    #[test]
    fn helo_is_set() {
        async_std::task::block_on(async move {
            let mut store = Store::default();
            let mut smtp = SmtpSession::default();
            let mut set = SmtpContext::new(&mut store, &mut smtp);

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
        let mut store = Store::default();
        let mut smtp = SmtpSession::default();
        let mut set = SmtpContext::new(&mut store, &mut smtp);

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
