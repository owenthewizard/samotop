use crate::{
    common::S1Fut,
    smtp::{
        apply_helo,
        command::{SmtpCommand, SmtpHelo, SmtpUnknownCommand},
        Action, Esmtp, Lmtp, SmtpContext,
    },
};

impl Action<SmtpHelo> for Lmtp {
    /// Applies given helo to the state
    /// It asserts the right HELO/EHLO variant
    fn apply<'a, 's, 'f>(&'a self, cmd: SmtpHelo, state: &'s mut SmtpContext) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
            match cmd.verb.to_ascii_uppercase().as_str() {
                "LHLO" => apply_helo(cmd, true, state),
                _ => Esmtp.apply(SmtpUnknownCommand::default(), state).await,
            }
        })
    }
}

impl Action<SmtpCommand> for Lmtp {
    fn apply<'a, 's, 'f>(&'a self, cmd: SmtpCommand, state: &'s mut SmtpContext) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
            use SmtpCommand as C;
            match cmd {
                C::Helo(helo) => Lmtp.apply(helo, state).await,
                cmd => Esmtp.apply(cmd, state).await,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::Recipient,
        smtp::{command::SmtpMail, SmtpHost, SmtpPath},
    };

    #[test]
    fn transaction_gets_reset() {
        async_std::task::block_on(async move {
            let mut set = SmtpContext::default();
            set.session.transaction.id = "someid".to_owned();
            set.session.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
            set.session.transaction.rcpts.push(Recipient::null());
            set.session
                .transaction
                .extra_headers
                .insert_str(0, "feeeha");

            Lmtp.apply(
                SmtpHelo {
                    verb: "LHLO".to_string(),
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
            let mut set = SmtpContext::default();

            Lmtp.apply(
                SmtpHelo {
                    verb: "LHLO".to_string(),
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
        async_std::task::block_on(async move {
            let mut set = SmtpContext::default();
            let res = Lmtp.apply(
                SmtpHelo {
                    verb: "LHLO".to_string(),
                    host: SmtpHost::Domain("wex.xor.ro".to_owned()),
                },
                &mut set,
            );

            is_send(res);
        })
    }

    fn is_send<T: Send>(_subj: T) {}
}
