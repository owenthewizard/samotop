use crate::{
    mail::{AddRecipientFailure, MailService, SessionInfo, StartMailFailure, Transaction},
    parser::Parser,
    smtp::{CodecControl, SmtpHelo, SmtpPath, SmtpReply},
};
use std::collections::VecDeque;

use super::SmtpSessionCommand;

pub struct SmtpState {
    pub service: Box<dyn SyncMailService>,
    pub session: SessionInfo,
    pub transaction: Transaction,
    pub writes: VecDeque<CodecControl>,
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

    pub fn reset(&mut self) {
        self.transaction = Transaction::default();
    }
}

impl SmtpState {
    //TODO: split say into action and response
    //fn start_tls(&mut self) -> Pin<Box<dyn Future<Output = Result<()>>>>;

    pub fn say(&mut self, what: CodecControl) -> SayResult {
        self.writes.push_back(what);
    }
    pub fn say_reply(&mut self, c: SmtpReply) -> SayResult {
        self.say(CodecControl::Response(c.to_string().into()))
    }
    pub fn say_ok(&mut self) -> SayResult {
        self.say_reply(SmtpReply::OkInfo)
    }
    pub fn say_ok_info(&mut self, info: String) -> SayResult {
        self.say_reply(SmtpReply::OkMessageInfo(info))
    }
    pub fn say_not_implemented(&mut self) -> SayResult {
        self.say_reply(SmtpReply::CommandNotImplementedFailure)
    }
    pub fn say_invalid_syntax(&mut self) -> SayResult {
        self.say_reply(SmtpReply::CommandSyntaxFailure)
    }
    pub fn say_command_sequence_fail(&mut self) -> SayResult {
        self.say_reply(SmtpReply::CommandSequenceFailure)
    }
    pub fn say_service_ready(&mut self, name: String) -> SayResult {
        self.say_reply(SmtpReply::ServiceReadyInfo(name))
    }
    pub fn say_helo(&mut self, local: String, remote: String) -> SayResult {
        self.say_reply(SmtpReply::OkHeloInfo {
            local,
            remote,
            extensions: vec![],
        })
    }
    pub fn say_ehlo(
        &mut self,
        local: String,
        extensions: Vec<String>,
        remote: String,
    ) -> SayResult {
        self.say_reply(SmtpReply::OkHeloInfo {
            local,
            remote,
            extensions,
        })
    }
    pub fn say_shutdown(&mut self, reply: SmtpReply) -> SayResult {
        self.say_reply(reply);
        self.say(CodecControl::Shutdown);
    }
    pub fn say_shutdown_err(&mut self, description: String) -> SayResult {
        self.say_shutdown(SmtpReply::ServiceNotAvailableError(description))
    }
    pub fn say_shutdown_ok(&mut self, description: String) -> SayResult {
        self.say_shutdown(SmtpReply::ClosingConnectionInfo(description))
    }
    pub fn say_mail_failed(&mut self, failure: StartMailFailure, description: String) -> SayResult {
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
    pub fn say_rcpt_failed(
        &mut self,
        failure: AddRecipientFailure,
        _description: String,
    ) -> SayResult {
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
    pub fn say_ok_recipient_not_local(&mut self, path: SmtpPath) -> SayResult {
        self.say_reply(SmtpReply::UserNotLocalInfo(format!("{}", path)))
    }
    pub fn say_mail_queue_refused(&mut self) -> SayResult {
        self.say_reply(SmtpReply::MailboxNotAvailableFailure)
    }
    pub fn say_start_data_challenge(&mut self, parser: Box<dyn Parser + Sync + Send>) -> SayResult {
        self.say_reply(SmtpReply::StartMailInputChallenge);
        self.say(CodecControl::Parser(parser));
    }
    pub fn say_start_tls(&mut self, name: String) -> SayResult {
        // TODO: better message response
        self.say_reply(SmtpReply::ServiceReadyInfo(name));
        self.say(CodecControl::StartTls);
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
        smtp::{SmtpMail, SmtpPath},
    };
    use futures_await_test::async_test;

    #[async_test]
    async fn transaction_gets_reset() {
        let mut sut = SmtpState::new(Builder::default());
        sut.transaction.id = "someid".to_owned();
        sut.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
        sut.transaction.rcpts.push(Recipient::null());
        sut.transaction.extra_headers.insert_str(0, "feeeha");
        sut.reset();
        assert!(sut.transaction.is_empty());
    }
}
