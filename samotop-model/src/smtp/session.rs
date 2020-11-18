use crate::{
    mail::{
        AddRecipientFailure, DefaultMailService, MailService, SessionInfo, StartMailFailure,
        Transaction,
    },
    smtp::{ExtensionSet, SmtpHelo, SmtpPath, SmtpReply, WriteControl},
};
use std::collections::VecDeque;

pub type SayResult = ();

pub trait SyncMailService: MailService + Sync + Send {}
impl<T> SyncMailService for T where T: MailService + Sync + Send {}

pub trait SmtpState: Send + Sync {
    fn reset_helo(&mut self, helo: SmtpHelo);
    fn reset(&mut self);
    fn transaction(&self) -> &Transaction;
    fn transaction_mut(&mut self) -> &mut Transaction;
    fn extensions(&self) -> &ExtensionSet;
    fn extensions_mut(&mut self) -> &mut ExtensionSet;
    fn session(&self) -> &SessionInfo;
    fn session_mut(&mut self) -> &mut SessionInfo;
    fn service(&self) -> &dyn SyncMailService;
    fn pop(&mut self) -> Option<WriteControl>;

    #[must_use = "future must be polled"]
    fn say(&mut self, what: WriteControl) -> SayResult;
    //TODO: split say into action and response
    //fn start_tls(&mut self) -> Pin<Box<dyn Future<Output = Result<()>>>>;

    fn say_reply(&mut self, c: SmtpReply) -> SayResult {
        self.say(WriteControl::Reply(c))
    }

    fn say_ok(&mut self) -> SayResult {
        self.say_reply(SmtpReply::OkInfo)
    }
    fn say_ok_info(&mut self, info: String) -> SayResult {
        self.say_reply(SmtpReply::OkMessageInfo(info))
    }
    fn say_not_implemented(&mut self) -> SayResult {
        self.say_reply(SmtpReply::CommandNotImplementedFailure)
    }
    fn say_command_sequence_fail(&mut self) -> SayResult {
        self.say_reply(SmtpReply::CommandSequenceFailure)
    }
    fn say_service_ready(&mut self, name: String) -> SayResult {
        self.say_reply(SmtpReply::ServiceReadyInfo(name))
    }
    fn say_helo(&mut self, local: String, remote: String) -> SayResult {
        self.say_reply(SmtpReply::OkHeloInfo {
            local,
            remote,
            extensions: vec![],
        })
    }
    fn say_ehlo(&mut self, local: String, extensions: Vec<String>, remote: String) -> SayResult {
        self.say_reply(SmtpReply::OkHeloInfo {
            local,
            remote,
            extensions,
        })
    }
    fn say_shutdown_err(&mut self, description: String) -> SayResult {
        self.say(WriteControl::Shutdown(SmtpReply::ServiceNotAvailableError(
            description,
        )))
    }
    fn say_shutdown_ok(&mut self, description: String) -> SayResult {
        self.say(WriteControl::Shutdown(SmtpReply::ClosingConnectionInfo(
            description,
        )))
    }
    fn say_mail_failed(&mut self, failure: StartMailFailure, description: String) -> SayResult {
        use StartMailFailure as F;
        match failure {
            F::TerminateSession => self.say_shutdown_err(description),
            F::Rejected => self.say_reply(SmtpReply::MailboxNotAvailableFailure),
            F::InvalidSender => self.say_reply(SmtpReply::MailboxNameInvalidFailure),
            F::InvalidParameter => self.say_reply(SmtpReply::UnknownMailParametersFailure),
            F::InvalidParameterValue => self.say_reply(SmtpReply::ParametersNotAccommodatedError),
            F::StorageExhaustedPermanently => self.say_reply(SmtpReply::StorageFailure),
            F::StorageExhaustedTemporarily => self.say_reply(SmtpReply::StorageError),
            F::FailedTemporarily => self.say_reply(SmtpReply::ProcesingError),
        }
    }
    fn say_rcpt_failed(&mut self, failure: AddRecipientFailure, _description: String) -> SayResult {
        use AddRecipientFailure as F;
        match failure {
            F::Moved(path) => self.say_reply(SmtpReply::UserNotLocalFailure(format!("{}", path))),
            F::RejectedPermanently => self.say_reply(SmtpReply::MailboxNotAvailableFailure),
            F::RejectedTemporarily => self.say_reply(SmtpReply::MailboxNotAvailableError),
            F::InvalidRecipient => self.say_reply(SmtpReply::MailboxNameInvalidFailure),
            F::InvalidParameter => self.say_reply(SmtpReply::UnknownMailParametersFailure),
            F::InvalidParameterValue => self.say_reply(SmtpReply::ParametersNotAccommodatedError),
            F::StorageExhaustedPermanently => self.say_reply(SmtpReply::StorageFailure),
            F::StorageExhaustedTemporarily => self.say_reply(SmtpReply::StorageError),
            F::FailedTemporarily => self.say_reply(SmtpReply::ProcesingError),
        }
    }
    fn say_ok_recipient_not_local(&mut self, path: SmtpPath) -> SayResult {
        self.say_reply(SmtpReply::UserNotLocalInfo(format!("{}", path)))
    }
    fn say_mail_queue_refused(&mut self) -> SayResult {
        self.say_reply(SmtpReply::MailboxNotAvailableFailure)
    }
    fn say_start_data_challenge(&mut self) -> SayResult {
        self.say(WriteControl::StartData(SmtpReply::StartMailInputChallenge))
    }
    fn say_start_tls(&mut self, name: String) -> SayResult {
        // TODO: better message response
        self.say(WriteControl::StartTls(SmtpReply::ServiceReadyInfo(name)))
    }
    fn say_mail_queue_failed_temporarily(&mut self) -> SayResult {
        self.say_reply(SmtpReply::MailboxNotAvailableError)
    }
    fn say_mail_queued(&mut self, id: &str) -> SayResult {
        let info = format!("Queued as {}", id);
        self.say_ok_info(info)
    }
}

pub struct SmtpStateBase {
    session: SessionInfo,
    transaction: Transaction,
    writes: VecDeque<WriteControl>,
    service: Box<dyn SyncMailService>,
}

impl Default for SmtpStateBase {
    fn default() -> Self {
        SmtpStateBase {
            service: Box::new(DefaultMailService::default()),
            writes: Default::default(),
            transaction: Default::default(),
            session: Default::default(),
        }
    }
}

impl SmtpState for SmtpStateBase {
    fn reset_helo(&mut self, helo: SmtpHelo) {
        self.reset();
        self.session.smtp_helo = Some(helo);
    }

    fn reset(&mut self) {
        self.transaction = Transaction::default();
    }

    fn transaction(&self) -> &Transaction {
        &self.transaction
    }

    fn transaction_mut(&mut self) -> &mut Transaction {
        &mut self.transaction
    }

    fn extensions(&self) -> &ExtensionSet {
        &self.session.extensions
    }

    fn extensions_mut(&mut self) -> &mut ExtensionSet {
        &mut self.session.extensions
    }

    fn session(&self) -> &SessionInfo {
        &self.session
    }

    fn session_mut(&mut self) -> &mut SessionInfo {
        &mut self.session
    }

    fn say(&mut self, what: WriteControl) {
        self.writes.push_back(what);
    }

    fn service(&self) -> &dyn SyncMailService {
        self.service.as_ref()
    }

    fn pop(&mut self) -> Option<WriteControl> {
        self.writes.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::smtp::{SmtpMail, SmtpPath, SmtpStateBase};
    use futures_await_test::async_test;

    #[async_test]
    async fn transaction_gets_reset() {
        let mut sut = SmtpStateBase::default();
        sut.transaction_mut().id = "someid".to_owned();
        sut.transaction_mut().mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
        sut.transaction_mut().rcpts.push(SmtpPath::Null);
        sut.transaction_mut().extra_headers.insert_str(0, "feeeha");
        sut.reset();
        assert!(sut.transaction().is_empty());
    }
}
