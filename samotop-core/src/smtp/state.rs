use crate::{
    mail::{AddRecipientFailure, Builder, MailService, SessionInfo, StartMailFailure, Transaction},
    smtp::{DriverControl, SmtpPath, SmtpReply},
};
use std::collections::VecDeque;

pub struct SmtpState {
    pub service: Box<dyn SyncMailService>,
    pub session: SessionInfo,
    pub transaction: Transaction,
    pub writes: VecDeque<DriverControl>,
}

impl Default for SmtpState {
    fn default() -> Self {
        SmtpState {
            service: Box::new(Builder::default().build()),
            session: SessionInfo::default(),
            transaction: Transaction::default(),
            writes: vec![].into(),
        }
    }
}

impl SmtpState {
    pub fn new(service: impl MailService + Send + Sync + 'static) -> Self {
        Self {
            service: Box::new(service),
            writes: Default::default(),
            transaction: Default::default(),
            session: Default::default(),
        }
    }
    pub fn reset_helo(&mut self, peer_name: String) {
        self.reset();
        self.session.peer_name = Some(peer_name);
    }

    pub fn reset(&mut self) -> SayResult {
        self.transaction = Transaction::default();
    }

    /// Shut the session down without a response
    pub fn shutdown(&mut self) -> SayResult {
        self.reset();
        self.session = SessionInfo::default();
        self.say(DriverControl::Shutdown)
    }
}

impl SmtpState {
    pub fn say(&mut self, what: DriverControl) -> SayResult {
        self.writes.push_back(what);
    }
    pub fn say_reply(&mut self, c: SmtpReply) -> SayResult {
        self.say(DriverControl::Response(c.to_string().into()))
    }
    /// Reply "250 Ok"
    pub fn say_ok(&mut self) -> SayResult {
        self.say_reply(SmtpReply::OkInfo)
    }
    /// Reply "250 @info"
    pub fn say_ok_info(&mut self, info: String) -> SayResult {
        self.say_reply(SmtpReply::OkMessageInfo(info))
    }
    /// Reply "502 Not implemented"
    pub fn say_not_implemented(&mut self) -> SayResult {
        self.say_reply(SmtpReply::CommandNotImplementedFailure)
    }
    /// Reply "500 Syntax error"
    pub fn say_invalid_syntax(&mut self) -> SayResult {
        self.say_reply(SmtpReply::CommandSyntaxFailure)
    }
    /// Reply "503 Command sequence error"
    pub fn say_command_sequence_fail(&mut self) -> SayResult {
        self.say_reply(SmtpReply::CommandSequenceFailure)
    }
    /// Reply "220 @name service ready"
    pub fn say_service_ready(&mut self) -> SayResult {
        // TODO - indicate ESMTP if available
        self.say_reply(SmtpReply::ServiceReadyInfo(
            self.session.service_name.clone(),
        ))
    }
    /// Reply something like "250 @local greets @remote"
    pub fn say_helo(&mut self) -> SayResult {
        self.say_reply(SmtpReply::OkHeloInfo {
            local: self.session.service_name.clone(),
            remote: self
                .session
                .peer_name
                .as_ref()
                .unwrap_or(&self.session.connection.peer_addr)
                .clone(),
            extensions: vec![],
        })
    }
    /// Reply something like "250 @local greets @remote, we have extensions: <extensions>"
    pub fn say_ehlo(&mut self) -> SayResult {
        self.say_reply(SmtpReply::OkHeloInfo {
            local: self.session.service_name.clone(),
            remote: self
                .session
                .peer_name
                .as_ref()
                .unwrap_or(&self.session.connection.peer_addr)
                .clone(),

            extensions: self.session.extensions.iter().map(String::from).collect(),
        })
    }
    /// Reply and shut the session down
    pub fn say_shutdown(&mut self, reply: SmtpReply) -> SayResult {
        self.say_reply(reply);
        self.shutdown()
    }
    /// Reply "421 @name service not available, closing transmission channel" and shut the session down
    pub fn say_shutdown_timeout(&mut self) -> SayResult {
        warn!("Timeout expired.");
        self.say_shutdown_service_err()
    }
    /// Reply "421 @name service not available, closing transmission channel" and shut the session down
    pub fn say_shutdown_service_err(&mut self) -> SayResult {
        self.say_shutdown(SmtpReply::ServiceNotAvailableError(
            self.session.service_name.clone(),
        ))
    }
    /// Processing error
    pub fn say_shutdown_processing_err(&mut self, description: String) -> SayResult {
        error!("Processing error: {}", description);
        self.say_shutdown(SmtpReply::ProcesingError)
    }
    /// Normal response to quit command
    pub fn say_shutdown_ok(&mut self) -> SayResult {
        self.say_shutdown(SmtpReply::ClosingConnectionInfo(
            self.session.service_name.clone(),
        ))
    }
    pub fn say_mail_failed(&mut self, failure: StartMailFailure, description: String) -> SayResult {
        use StartMailFailure as F;
        error!("Sending mail failed: {:?}, {}", failure, description);
        match failure {
            F::TerminateSession => self.say_shutdown_service_err(),
            F::Rejected => self.say_reply(SmtpReply::MailboxNotAvailableFailure),
            F::InvalidSender => self.say_reply(SmtpReply::MailboxNameInvalidFailure),
            F::InvalidParameter => self.say_reply(SmtpReply::UnknownMailParametersFailure),
            F::InvalidParameterValue => self.say_reply(SmtpReply::ParametersNotAccommodatedError),
            F::StorageExhaustedPermanently => self.say_reply(SmtpReply::StorageFailure),
            F::StorageExhaustedTemporarily => self.say_reply(SmtpReply::StorageError),
            F::FailedTemporarily => self.say_reply(SmtpReply::ProcesingError),
        }
    }
    pub fn say_rcpt_failed(
        &mut self,
        failure: AddRecipientFailure,
        description: String,
    ) -> SayResult {
        use AddRecipientFailure as F;
        error!("Adding RCPT failed: {:?}, {}", failure, description);
        match failure {
            F::TerminateSession => self.say_shutdown_service_err(),
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
    pub fn say_ok_recipient_not_local(&mut self, path: SmtpPath) -> SayResult {
        self.say_reply(SmtpReply::UserNotLocalInfo(format!("{}", path)))
    }
    pub fn say_mail_queue_refused(&mut self) -> SayResult {
        self.say_reply(SmtpReply::MailboxNotAvailableFailure)
    }
    pub fn say_start_data_challenge(&mut self) -> SayResult {
        self.say_reply(SmtpReply::StartMailInputChallenge);
        self.transaction.mode = Some(Transaction::DATA_MODE);
    }
    pub fn say_start_tls(&mut self) -> SayResult {
        self.say_service_ready();
        self.say(DriverControl::StartTls);
    }
    pub fn say_mail_queue_failed_temporarily(&mut self) -> SayResult {
        self.say_reply(SmtpReply::MailboxNotAvailableError)
    }
    pub fn say_mail_queued(&mut self, id: &str) -> SayResult {
        let info = format!("Queued as {}", id);
        self.say_ok_info(info)
    }
}

type SayResult = ();

pub trait SyncMailService: MailService + Sync + Send {}
impl<T> SyncMailService for T where T: MailService + Sync + Send {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::{Builder, Recipient},
        smtp::{command::SmtpMail, SmtpPath},
    };

    #[test]
    fn transaction_gets_reset() {
        let mut sut = SmtpState::new(Builder::default().build());
        sut.transaction.id = "someid".to_owned();
        sut.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
        sut.transaction.rcpts.push(Recipient::null());
        sut.transaction.extra_headers.insert_str(0, "feeeha");
        sut.reset();
        assert!(sut.transaction.is_empty());
    }
}
