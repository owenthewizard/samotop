use super::{EsmtpCommand, Rfc5321};
use crate::smtp::{ApplyCommand, SmtpData, SmtpSessionCommand, SmtpState};
use crate::{common::*, mail::DispatchError};

impl SmtpSessionCommand for EsmtpCommand<SmtpData> {
    fn verb(&self) -> &str {
        "DATA"
    }

    fn apply(&self, state: SmtpState) -> S2Fut<SmtpState> {
        Rfc5321::apply_cmd(&self.instruction, state)
    }
}

impl ApplyCommand<SmtpData> for Rfc5321 {
    fn apply_cmd(_cmd: &SmtpData, mut state: SmtpState) -> S2Fut<SmtpState> {
        if state.transaction.id.is_empty()
            || state.session.peer_name.is_none()
            || state.transaction.mail.is_none()
            || state.transaction.rcpts.is_empty()
        {
            state.say_command_sequence_fail();
            return Box::pin(ready(state));
        }
        let transaction = std::mem::take(&mut state.transaction);
        let fut = async move {
            match state.service.send_mail(&state.session, transaction).await {
                Ok(transaction) if transaction.sink.is_none() => {
                    warn!(
                        "Send_mail returned OK message without sink for transaction {}",
                        transaction.id
                    );
                    state.say_mail_queue_failed_temporarily();
                }
                Ok(transaction) => {
                    state.say_start_data_challenge(state.service.get_parser_for_data());
                    state.transaction = transaction;
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
        mail::{Builder, Recipient},
        smtp::{CodecControl, SmtpMail, SmtpPath},
    };
    use futures_await_test::async_test;

    #[async_test]
    async fn sink_gets_set() {
        let mut set = SmtpState::new(Builder::default());
        set.session.peer_name = Some("xx.io".to_owned());
        set.transaction.id = "someid".to_owned();
        set.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
        set.transaction.rcpts.push(Recipient::null());
        set.transaction.extra_headers.insert_str(0, "feeeha");
        let sink: Vec<u8> = vec![];
        set.transaction.sink = Some(Box::pin(sink));
        let sut = Rfc5321::command(SmtpData);
        let mut res = sut.apply(set).await;
        match res.writes.pop_front() {
            Some(CodecControl::Response(bytes)) if bytes.starts_with(b"354 ") => {}
            otherwise => panic!("Expected mail data input challenge, got {:?}", otherwise),
        }

        assert!(res.transaction.sink.is_some());
    }

    #[async_test]
    async fn command_sequence_is_assured_missing_helo() {
        let set = SmtpState::new(Builder::default());
        let sut = Rfc5321::command(SmtpData);
        let mut res = sut.apply(set).await;
        match res.writes.pop_front() {
            Some(CodecControl::Response(bytes)) if bytes.starts_with(b"503 ") => {}
            otherwise => panic!("Expected command sequence failure, got {:?}", otherwise),
        }
        assert!(res.transaction.sink.is_none());
    }

    #[async_test]
    async fn command_sequence_is_assured_missing_mail() {
        let mut set = SmtpState::new(Builder::default());
        set.session.peer_name = Some("xx.iu".to_owned());
        let sut = Rfc5321::command(SmtpData);
        let mut res = sut.apply(set).await;
        match res.writes.pop_front() {
            Some(CodecControl::Response(bytes)) if bytes.starts_with(b"503 ") => {}
            otherwise => panic!("Expected command sequence failure, got {:?}", otherwise),
        }
        assert!(res.transaction.sink.is_none());
    }
    #[async_test]
    async fn command_sequence_is_assured_missing_rcpt() {
        let mut set = SmtpState::new(Builder::default());
        set.session.peer_name = Some("xx.iu".to_owned());
        set.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
        let sut = Rfc5321::command(SmtpData);
        let mut res = sut.apply(set).await;
        match res.writes.pop_front() {
            Some(CodecControl::Response(bytes)) if bytes.starts_with(b"503 ") => {}
            otherwise => panic!("Expected command sequence failure, got {:?}", otherwise),
        }
        assert!(res.transaction.sink.is_none());
    }
}
