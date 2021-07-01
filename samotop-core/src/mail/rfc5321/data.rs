use super::Esmtp;
use crate::mail::DispatchError;
use crate::smtp::{command::SmtpData, Action, SmtpState};

#[async_trait::async_trait]
impl Action<SmtpData> for Esmtp {
    async fn apply(&self, _cmd: SmtpData, state: &mut SmtpState) {
        if state.transaction.id.is_empty()
            || state.session.peer_name.is_none()
            || state.transaction.mail.is_none()
            || state.transaction.rcpts.is_empty()
        {
            state.say_command_sequence_fail();
            return;
        }
        let transaction = std::mem::take(&mut state.transaction);

        match state.service.send_mail(&state.session, transaction).await {
            Ok(transaction) if transaction.sink.is_none() => {
                warn!(
                    "Send_mail returned OK message without sink for transaction {}",
                    transaction.id
                );
                state.say_mail_queue_failed_temporarily();
            }
            Ok(transaction) => {
                state.say_start_data_challenge();
                state.transaction = transaction;
            }
            Err(DispatchError::Refused) => {
                state.say_mail_queue_refused();
            }
            Err(DispatchError::FailedTemporarily) => {
                state.say_mail_queue_failed_temporarily();
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::{Builder, Recipient},
        smtp::{command::SmtpMail, DriverControl, SmtpPath},
    };
    use futures_await_test::async_test;

    #[async_test]
    async fn sink_gets_set() {
        let mut set = SmtpState::new(Builder::default().into_service());
        set.session.peer_name = Some("xx.io".to_owned());
        set.transaction.id = "someid".to_owned();
        set.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
        set.transaction.rcpts.push(Recipient::null());
        set.transaction.extra_headers.insert_str(0, "feeeha");
        let sink: Vec<u8> = vec![];
        set.transaction.sink = Some(Box::pin(sink));

        Esmtp.apply(SmtpData, &mut set).await;
        match set.writes.pop_front() {
            Some(DriverControl::Response(bytes)) if bytes.starts_with(b"354 ") => {}
            otherwise => panic!("Expected mail data input challenge, got {:?}", otherwise),
        }

        assert!(set.transaction.sink.is_some());
    }

    #[async_test]
    async fn command_sequence_is_assured_missing_helo() {
        let mut set = SmtpState::new(Builder::default().into_service());

        Esmtp.apply(SmtpData, &mut set).await;
        match set.writes.pop_front() {
            Some(DriverControl::Response(bytes)) if bytes.starts_with(b"503 ") => {}
            otherwise => panic!("Expected command sequence failure, got {:?}", otherwise),
        }
        assert!(set.transaction.sink.is_none());
    }

    #[async_test]
    async fn command_sequence_is_assured_missing_mail() {
        let mut set = SmtpState::new(Builder::default().into_service());
        set.session.peer_name = Some("xx.iu".to_owned());

        Esmtp.apply(SmtpData, &mut set).await;
        match set.writes.pop_front() {
            Some(DriverControl::Response(bytes)) if bytes.starts_with(b"503 ") => {}
            otherwise => panic!("Expected command sequence failure, got {:?}", otherwise),
        }
        assert!(set.transaction.sink.is_none());
    }
    #[async_test]
    async fn command_sequence_is_assured_missing_rcpt() {
        let mut set = SmtpState::new(Builder::default().into_service());
        set.session.peer_name = Some("xx.iu".to_owned());
        set.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));

        Esmtp.apply(SmtpData, &mut set).await;
        match set.writes.pop_front() {
            Some(DriverControl::Response(bytes)) if bytes.starts_with(b"503 ") => {}
            otherwise => panic!("Expected command sequence failure, got {:?}", otherwise),
        }
        assert!(set.transaction.sink.is_none());
    }
}
