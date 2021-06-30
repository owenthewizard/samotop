use crate::{
    common::*,
    mail::{apply_helo, Rfc2033, Rfc5321},
    smtp::{ApplyCommand, SmtpHelo, SmtpSessionCommand, SmtpState, SmtpUnknownCommand},
};

use super::LmtpCommand;

impl SmtpSessionCommand for LmtpCommand<SmtpHelo> {
    fn verb(&self) -> &str {
        self.instruction.verb.as_str()
    }

    fn apply(&self, state: SmtpState) -> S1Fut<SmtpState> {
        Rfc2033::apply_cmd(&self.instruction, state)
    }
}

impl ApplyCommand<SmtpHelo> for Rfc2033 {
    /// Applies given helo to the state
    /// It asserts the right HELO/EHLO variant
    fn apply_cmd(helo: &SmtpHelo, state: SmtpState) -> S1Fut<SmtpState> {
        Box::pin(async move {
            match helo.verb.to_ascii_uppercase().as_str() {
                "LHLO" => apply_helo(helo, true, state).await,
                _ => {
                    Rfc5321::command(SmtpUnknownCommand::default())
                        .apply(state)
                        .await
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::{Builder, Recipient},
        smtp::{SmtpHost, SmtpMail, SmtpPath},
    };
    use futures_await_test::async_test;

    #[async_test]
    async fn transaction_gets_reset() {
        let mut set = SmtpState::new(Builder::default());
        set.transaction.id = "someid".to_owned();
        set.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
        set.transaction.rcpts.push(Recipient::null());
        set.transaction.extra_headers.insert_str(0, "feeeha");
        let sut = Rfc2033::command(SmtpHelo {
            verb: "LHLO".to_string(),
            host: SmtpHost::Domain("wex.xor.ro".to_owned()),
        });
        let res = sut.apply(set).await;
        assert!(res.transaction.is_empty());
    }

    #[async_test]
    async fn helo_is_set() {
        let set = SmtpState::new(Builder::default());
        let sut = Rfc2033::command(SmtpHelo {
            verb: "LHLO".to_string(),
            host: SmtpHost::Domain("wex.xor.ro".to_owned()),
        });
        let res = sut.apply(set).await;
        assert_eq!(res.session.peer_name, Some("wex.xor.ro".to_owned()));
    }

    #[test]
    fn is_sync_and_send() {
        let sut = Rfc2033::command(SmtpHelo {
            verb: "LHLO".to_string(),
            host: SmtpHost::Domain("wex.xor.ro".to_owned()),
        });
        let set = SmtpState::new(Builder::default());
        let res = sut.apply(set);

        is_send(res);
    }

    fn is_send<T: Send>(_subj: T) {}
}
