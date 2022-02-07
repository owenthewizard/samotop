use super::Esmtp;
use crate::common::S2Fut;
use crate::mail::{DispatchError, MailDispatchService};
use crate::smtp::{command::SmtpData, Action, SmtpContext};

impl Action<SmtpData> for Esmtp {
    fn apply<'a, 's, 'f>(&'a self, _cmd: SmtpData, state: &'s mut SmtpContext) -> S2Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
            if state.session.transaction.id.is_empty()
                || state.session.peer_name.is_none()
                || state.session.transaction.mail.is_none()
                || state.session.transaction.rcpts.is_empty()
            {
                state.session.reset();
                state.session.say_command_sequence_fail();
                return;
            }

            match state
                .store
                .get_or_compose::<MailDispatchService>()
                .open_mail_body(&mut state.session)
                .await
            {
                Ok(()) if state.session.transaction.sink.is_none() => {
                    warn!(
                        "Send_mail returned OK message without sink for transaction {}",
                        state.session.transaction.id
                    );
                    state.session.reset();
                    state.session.say_mail_queue_failed_temporarily();
                }
                Ok(()) => {
                    state.session.say_start_data_challenge();
                }
                Err(DispatchError::Permanent) => {
                    state.session.reset();
                    state.session.say_mail_queue_refused();
                }
                Err(DispatchError::Temporary) => {
                    state.session.reset();
                    state.session.say_mail_queue_failed_temporarily();
                }
            };
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::Recipient,
        smtp::{command::SmtpMail, DriverControl, SmtpPath, SmtpSession},
        store::Store,
    };

    #[test]
    fn sink_gets_set() {
        async_std::task::block_on(async move {
            let mut store = Store::default();
            let mut smtp = SmtpSession::default();
            let mut set = SmtpContext::new(&mut store, &mut smtp);

            set.session.peer_name = Some("xx.io".to_owned());
            set.session.transaction.id = "someid".to_owned();
            set.session.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
            set.session.transaction.rcpts.push(Recipient::null());
            set.session
                .transaction
                .extra_headers
                .insert_str(0, "feeeha");
            let sink: Vec<u8> = vec![];
            set.session.transaction.sink = Some(Box::pin(sink));

            Esmtp.apply(SmtpData, &mut set).await;
            match set.session.pop_control() {
                Some(DriverControl::Response(bytes)) if bytes.starts_with(b"354 ") => {}
                otherwise => panic!("Expected mail data input challenge, got {:?}", otherwise),
            }

            assert!(set.session.transaction.sink.is_some());
        })
    }

    #[test]
    fn command_sequence_is_assured_missing_helo() {
        async_std::task::block_on(async move {
            let mut store = Store::default();
            let mut smtp = SmtpSession::default();
            let mut set = SmtpContext::new(&mut store, &mut smtp);

            Esmtp.apply(SmtpData, &mut set).await;
            match set.session.pop_control() {
                Some(DriverControl::Response(bytes)) if bytes.starts_with(b"503 ") => {}
                otherwise => panic!("Expected command sequence failure, got {:?}", otherwise),
            }
            assert!(set.session.transaction.sink.is_none());
        })
    }

    #[test]
    fn command_sequence_is_assured_missing_mail() {
        async_std::task::block_on(async move {
            let mut store = Store::default();
            let mut smtp = SmtpSession::default();
            let mut set = SmtpContext::new(&mut store, &mut smtp);

            set.session.peer_name = Some("xx.iu".to_owned());

            Esmtp.apply(SmtpData, &mut set).await;
            match set.session.pop_control() {
                Some(DriverControl::Response(bytes)) if bytes.starts_with(b"503 ") => {}
                otherwise => panic!("Expected command sequence failure, got {:?}", otherwise),
            }
            assert!(set.session.transaction.sink.is_none());
        })
    }
    #[test]
    fn command_sequence_is_assured_missing_rcpt() {
        async_std::task::block_on(async move {
            let mut store = Store::default();
            let mut smtp = SmtpSession::default();
            let mut set = SmtpContext::new(&mut store, &mut smtp);

            set.session.peer_name = Some("xx.iu".to_owned());
            set.session.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));

            Esmtp.apply(SmtpData, &mut set).await;
            match set.session.pop_control() {
                Some(DriverControl::Response(bytes)) if bytes.starts_with(b"503 ") => {}
                otherwise => panic!("Expected command sequence failure, got {:?}", otherwise),
            }
            assert!(set.session.transaction.sink.is_none());
        })
    }
}
