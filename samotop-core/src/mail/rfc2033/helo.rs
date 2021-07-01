use crate::{
    mail::{apply_helo, Esmtp, Lmtp},
    smtp::{
        command::{SmtpHelo, SmtpUnknownCommand},
        Action, SmtpState,
    },
};

#[async_trait::async_trait]
impl Action<SmtpHelo> for Lmtp {
    /// Applies given helo to the state
    /// It asserts the right HELO/EHLO variant
    async fn apply(&self, cmd: SmtpHelo, state: &mut SmtpState) {
        match cmd.verb.to_ascii_uppercase().as_str() {
            "LHLO" => apply_helo(cmd, true, state),
            _ => Esmtp.apply(SmtpUnknownCommand::default(), state).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::{Builder, Recipient},
        smtp::{command::SmtpMail, SmtpHost, SmtpPath},
    };
    use futures_await_test::async_test;

    #[async_test]
    async fn transaction_gets_reset() {
        let mut set = SmtpState::new(Builder::default().into_service());
        set.transaction.id = "someid".to_owned();
        set.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
        set.transaction.rcpts.push(Recipient::null());
        set.transaction.extra_headers.insert_str(0, "feeeha");

        Lmtp.apply(
            SmtpHelo {
                verb: "LHLO".to_string(),
                host: SmtpHost::Domain("wex.xor.ro".to_owned()),
            },
            &mut set,
        )
        .await;
        assert!(set.transaction.is_empty());
    }

    #[async_test]
    async fn helo_is_set() {
        let mut set = SmtpState::new(Builder::default().into_service());

        Lmtp.apply(
            SmtpHelo {
                verb: "LHLO".to_string(),
                host: SmtpHost::Domain("wex.xor.ro".to_owned()),
            },
            &mut set,
        )
        .await;
        assert_eq!(set.session.peer_name, Some("wex.xor.ro".to_owned()));
    }

    #[test]
    fn is_sync_and_send() {
        let mut set = SmtpState::new(Builder::default().into_service());
        let res = Lmtp.apply(
            SmtpHelo {
                verb: "LHLO".to_string(),
                host: SmtpHost::Domain("wex.xor.ro".to_owned()),
            },
            &mut set,
        );

        is_send(res);
    }

    fn is_send<T: Send>(_subj: T) {}
}
