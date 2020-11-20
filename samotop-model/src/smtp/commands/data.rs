use crate::smtp::{SmtpSessionCommand, SmtpState};
use crate::{common::*, mail::DispatchError};

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct SmtpData;

impl SmtpSessionCommand for SmtpData {
    fn verb(&self) -> &str {
        "DATA"
    }
    fn apply<'s, 'f, S>(self, mut state: S) -> S2Fut<'f, S>
    where
        S: SmtpState + 's,
        's: 'f,
    {
        if state.transaction().id.is_empty()
            || state.session().smtp_helo.is_none()
            || state.transaction().mail.is_none()
            || state.transaction().rcpts.is_empty()
        {
            state.say_command_sequence_fail();
            return Box::pin(ready(state));
        }

        let transaction = std::mem::take(state.transaction_mut());
        let fut = async move {
            match state
                .service()
                .send_mail(state.session(), transaction)
                .await
            {
                Ok(transaction) => {
                    state.say_start_data_challenge();
                    *state.transaction_mut() = transaction;
                }
                Err(DispatchError::Refused) => {
                    state.say_mail_queue_refused();
                }
                Err(DispatchError::FailedTemporarily) => {
                    state.say_mail_queue_failed_temporarily();
                }
            };
            state
        };

        Box::pin(fut)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::Builder,
        smtp::{SmtpHelo, SmtpHost, SmtpMail, SmtpPath, SmtpReply, SmtpStateBase, WriteControl},
    };
    use futures_await_test::async_test;

    #[async_test]
    async fn sink_gets_set() {
        let mut set = SmtpStateBase::new(Builder::default());
        set.session_mut().smtp_helo = Some(SmtpHelo::Helo(SmtpHost::Domain("xx.io".to_owned())));
        set.transaction_mut().id = "someid".to_owned();
        set.transaction_mut().mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
        set.transaction_mut().rcpts.push(SmtpPath::Null);
        set.transaction_mut().extra_headers.insert_str(0, "feeeha");
        let sink: Vec<u8> = vec![];
        set.transaction_mut().sink = Some(Box::pin(sink));
        let sut = SmtpData;
        let mut res = sut.apply(set).await;
        assert_eq!(
            res.pop(),
            Some(WriteControl::StartData(SmtpReply::StartMailInputChallenge))
        );
        assert!(res.transaction().sink.is_some());
    }

    #[async_test]
    async fn command_sequence_is_assured_missing_helo() {
        let set = SmtpStateBase::new(Builder::default());
        let sut = SmtpData;
        let mut res = sut.apply(set).await;
        assert_eq!(
            res.pop(),
            Some(WriteControl::Reply(SmtpReply::CommandSequenceFailure))
        );
        assert!(res.transaction().sink.is_none());
    }

    #[async_test]
    async fn command_sequence_is_assured_missing_mail() {
        let mut set = SmtpStateBase::new(Builder::default());
        set.session_mut().smtp_helo = Some(SmtpHelo::Helo(SmtpHost::Domain("xx.io".to_owned())));
        let sut = SmtpData;
        let mut res = sut.apply(set).await;
        assert_eq!(
            res.pop(),
            Some(WriteControl::Reply(SmtpReply::CommandSequenceFailure))
        );
        assert!(res.transaction().sink.is_none());
    }
    #[async_test]
    async fn command_sequence_is_assured_missing_rcpt() {
        let mut set = SmtpStateBase::new(Builder::default());
        set.session_mut().smtp_helo = Some(SmtpHelo::Helo(SmtpHost::Domain("xx.io".to_owned())));
        set.transaction_mut().mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
        let sut = SmtpData;
        let mut res = sut.apply(set).await;
        assert_eq!(
            res.pop(),
            Some(WriteControl::Reply(SmtpReply::CommandSequenceFailure))
        );
        assert!(res.transaction().sink.is_none());
    }
}
