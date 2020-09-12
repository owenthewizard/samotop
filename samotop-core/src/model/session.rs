use crate::common::{Pin, Write};
use crate::model::mail::AddRecipientFailure;
use crate::model::mail::Envelope;
use crate::model::mail::SessionInfo;
use crate::model::mail::StartMailFailure;

use crate::model::io::*;
use crate::model::smtp::*;
use std::collections::VecDeque;

#[derive(Debug, Default)]
pub struct Buffers {
    pub answers: VecDeque<WriteControl>,
    pub state: State,
}
impl Buffers {
    // pub fn rst(&mut self) -> &mut Self {
    //     self.state = match std::mem::replace(&mut self.state, State::Closed) {
    //         State::Mail(m) => State::Connected(StateHelo::from(m)),
    //         other => other,
    //     };
    //     self
    // }
    pub fn say_ok(&mut self) -> &mut Self {
        self.say_reply(SmtpReply::OkInfo)
    }
    pub fn say_ok_info(&mut self, info: String) -> &mut Self {
        self.say_reply(SmtpReply::OkMessageInfo(info))
    }
    pub fn say_not_implemented(&mut self) -> &mut Self {
        self.say_reply(SmtpReply::CommandNotImplementedFailure)
    }
    pub fn say_command_sequence_fail(&mut self) -> &mut Self {
        self.say_reply(SmtpReply::CommandSequenceFailure)
    }
    pub fn say_service_ready(&mut self, name: &str) -> &mut Self {
        let name = name.to_owned();
        self.say_reply(SmtpReply::ServiceReadyInfo(name))
    }
    pub fn say_helo(&mut self, name: &str, remote: String) -> &mut Self {
        let local = name.to_owned();
        self.say_reply(SmtpReply::OkHeloInfo { local, remote })
    }
    pub fn say_ehlo(
        &mut self,
        name: &str,
        extensions: Vec<SmtpExtension>,
        remote: String,
    ) -> &mut Self {
        let local = name.to_owned();
        self.say_reply(SmtpReply::OkEhloInfo {
            local,
            remote,
            extensions,
        })
    }
    pub fn say_shutdown_err(&mut self, description: String) -> &mut Self {
        self.say(WriteControl::Shutdown(SmtpReply::ServiceNotAvailableError(
            description,
        )))
    }
    pub fn say_shutdown_ok(&mut self, description: String) -> &mut Self {
        self.say(WriteControl::Shutdown(SmtpReply::ClosingConnectionInfo(
            description,
        )))
    }
    pub fn say_mail_failed(&mut self, failure: StartMailFailure, description: String) -> &mut Self {
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
    ) -> &mut Self {
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
    pub fn say_ok_recipient_not_local(&mut self, path: SmtpPath) -> &mut Self {
        self.say_reply(SmtpReply::UserNotLocalInfo(format!("{}", path)))
    }
    pub fn say_mail_queue_refused(&mut self) -> &mut Self {
        self.say_reply(SmtpReply::MailboxNotAvailableFailure)
    }
    pub fn say_start_data_challenge(&mut self) -> &mut Self {
        self.say(WriteControl::StartData(SmtpReply::StartMailInputChallenge))
    }
    pub fn say_mail_queue_failed_temporarily(&mut self) -> &mut Self {
        self.say_reply(SmtpReply::MailboxNotAvailableError)
    }
    pub fn say_mail_queued(&mut self, id: &str) -> &mut Self {
        let info = format!("Queued as {}", id);
        self.say_ok_info(info)
    }
    pub fn say_reply(&mut self, c: SmtpReply) -> &mut Self {
        self.say(WriteControl::Reply(c))
    }
    pub fn say(&mut self, c: WriteControl) -> &mut Self {
        self.answers.push_back(c);
        self
    }
}
#[derive(Debug)]
pub enum State {
    New,
    Connected(SessionInfo),
    Mail(Envelope),
    Data(StateData),
    Closed,
}
impl Default for State {
    fn default() -> Self {
        State::New
    }
}
pub struct StateData {
    pub session: SessionInfo,
    pub mailid: String,
    pub sink: Pin<Box<dyn Write + Send + Sync + 'static>>,
}
impl std::fmt::Debug for StateData {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let nodebug = "_";
        f.debug_struct("StateData")
            .field("mailid", &self.mailid)
            .field("session", &self.session)
            .field("sink", &nodebug)
            .finish()
    }
}
impl<M: Write + Send + Sync + 'static> From<(Envelope, M)> for StateData {
    fn from(tuple: (Envelope, M)) -> Self {
        let (Envelope { session, id, .. }, sink) = tuple;
        StateData {
            session,
            mailid: id,
            sink: Box::pin(sink),
        }
    }
}
