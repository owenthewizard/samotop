use crate::mail::{AddRecipientFailure, StartMailFailure, Transaction};
use crate::smtp::*;

#[derive(Debug)]
pub struct SmtpSession {
    /// ESMTP extensions enabled for this session
    pub extensions: ExtensionSet,
    /// The name of the service serving this session
    pub service_name: String,
    /// The name of the peer as introduced by the HELO command
    pub peer_name: Option<String>,
    /// The address of the remote peer
    pub peer_addr: String,
    /// The address of the local endpoint
    pub local_addr: String,
    /// Output to be processed by a driver - responses and IO controls
    pub output: Vec<DriverControl>,
    /// Input to be interpretted
    pub input: Vec<u8>,
    /// Special mode used to switch parsers
    pub mode: Option<&'static str>,
    /// Current e-mail transaction
    pub transaction: Transaction,
}

impl Default for SmtpSession {
    fn default() -> Self {
        Self {
            extensions: Default::default(),
            service_name: "samotop".to_string(),
            peer_name: Default::default(),
            peer_addr: Default::default(),
            local_addr: Default::default(),
            output: Default::default(),
            input: Default::default(),
            mode: Default::default(),
            transaction: Default::default(),
        }
    }
}

impl SmtpSession {
    /// Special mode where classic SMTP data are expected,
    /// used after reading some data without CRLF to keep track of the dot state
    pub const DATA_PARTIAL_MODE: &'static str = "DATA_PARTIAL";
    /// Special mode where classic SMTP data are expected
    pub const DATA_MODE: &'static str = "DATA";

    pub fn new() -> Self {
        Self::default()
    }
    pub fn is_expecting_commands(&self) -> bool {
        self.mode.is_none() || self.transaction.sink.is_none()
    }
    pub fn reset_helo(&mut self, peer_name: String) {
        self.reset();
        self.peer_name = Some(peer_name);
    }

    pub fn reset(&mut self) -> SayResult {
        self.transaction = Transaction::default();
        self.mode = None;
    }

    /// Shut the session down without a response
    pub fn shutdown(&mut self) -> SayResult {
        self.reset();
        self.say(DriverControl::Shutdown)
    }
    pub fn pop_control(&mut self) -> Option<DriverControl> {
        if self.output.is_empty() {
            None
        } else {
            Some(self.output.remove(0))
        }
    }

    pub fn say(&mut self, what: DriverControl) -> SayResult {
        self.output.push(what);
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
        self.say_reply(SmtpReply::ServiceReadyInfo(self.service_name.clone()))
    }
    /// Reply something like "250 @local greets @remote"
    pub fn say_helo(&mut self) -> SayResult {
        self.say_reply(SmtpReply::OkHeloInfo {
            local: self.service_name.clone(),
            remote: self
                .peer_name
                .clone()
                .unwrap_or_else(|| self.peer_addr.clone()),
            extensions: vec![],
        })
    }
    /// Reply something like "250 @local greets @remote, we have extensions: <extensions>"
    pub fn say_ehlo(&mut self) -> SayResult {
        self.say_reply(SmtpReply::OkHeloInfo {
            local: self.service_name.clone(),
            remote: self
                .peer_name
                .clone()
                .unwrap_or_else(|| self.peer_addr.clone()),
            extensions: self.extensions.iter().map(String::from).collect(),
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
            self.service_name.clone(),
        ))
    }
    /// Processing error
    pub fn say_shutdown_processing_err(&mut self, description: String) -> SayResult {
        error!("Processing error: {}", description);
        self.say_shutdown(SmtpReply::ProcesingError)
    }
    /// Normal response to quit command
    pub fn say_shutdown_ok(&mut self) -> SayResult {
        self.say_shutdown(SmtpReply::ClosingConnectionInfo(self.service_name.clone()))
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
        self.mode = Some(Self::DATA_MODE);
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

impl std::fmt::Display for SmtpSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(
            f,
            "Client {:?} ({:?}) using service {} with extensions {} on {:?}. There are {} input bytes and {} output items pending.",
            self.peer_name,
            self.peer_addr,
            self.service_name,
            self.extensions
                .iter()
                .fold(String::new(), |s, r| s + format!("{}, ", r).as_ref()),
            self.local_addr,
            self.input.len(),
            self.output.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::Recipient,
        smtp::{command::SmtpMail, SmtpPath},
    };

    #[test]
    fn transaction_gets_reset() {
        let mut sut = SmtpSession::default();
        sut.transaction.id = "someid".to_owned();
        sut.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
        sut.transaction.rcpts.push(Recipient::null());
        sut.transaction.extra_headers.insert_str(0, "feeeha");
        sut.reset();
        assert!(sut.transaction.is_empty());
    }
}
