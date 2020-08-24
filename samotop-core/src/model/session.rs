use crate::common::{Error, Pin, Sink};

use crate::model::io::*;
use crate::model::smtp::*;
use std::collections::VecDeque;

#[derive(Debug, Default)]
pub struct Buffers {
    pub answers: VecDeque<WriteControl>,
    pub state: State,
}
impl Buffers {
    pub fn rst(&mut self) -> &mut Self {
        self.state = match std::mem::replace(&mut self.state, State::Closed) {
            State::Mail(m) => State::Connected(StateHelo::from(m)),
            other => other,
        };
        self
    }
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
    pub fn say_recipient_not_accepted(&mut self) -> &mut Self {
        self.say_reply(SmtpReply::MailboxNotAvailableFailure)
    }
    pub fn say_recipient_not_local(&mut self, path: SmtpPath) -> &mut Self {
        self.say_reply(SmtpReply::UserNotLocalFailure(format!("{}", path)))
    }
    pub fn say_ok_recipient_not_local(&mut self, path: SmtpPath) -> &mut Self {
        self.say_reply(SmtpReply::UserNotLocalInfo(format!("{}", path)))
    }
    pub fn say_mail_queue_refused(&mut self) -> &mut Self {
        self.say_reply(SmtpReply::MailboxNotAvailableFailure)
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
    Connected(StateHelo),
    Mail(StateMail),
    Data(StateData),
    Closed,
}
impl Default for State {
    fn default() -> Self {
        State::New
    }
}
#[derive(Debug)]
pub struct StateHelo {
    pub connection: Connection,
    pub peer_helo: Option<SmtpHelo>,
}
#[derive(Debug)]
pub struct StateMail {
    pub connection: Connection,
    pub peer_helo: Option<SmtpHelo>,
    pub mailid: String,
    pub mail: SmtpMail,
    pub recipients: Vec<SmtpPath>,
}
pub struct StateData {
    pub connection: Connection,
    pub peer_helo: Option<SmtpHelo>,
    pub mailid: String,
    pub sink: Pin<Box<dyn Sink<Vec<u8>, Error = Error> + Send + Sync + 'static>>,
}
impl std::fmt::Debug for StateData {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("StateData")
    }
}
/// This happens as part of reset
impl From<StateMail> for StateHelo {
    fn from(state: StateMail) -> Self {
        Self {
            connection: state.connection,
            peer_helo: state.peer_helo,
        }
    }
}
/// This happens as part of reset
impl From<StateData> for StateHelo {
    fn from(state: StateData) -> Self {
        Self {
            connection: state.connection,
            peer_helo: state.peer_helo,
        }
    }
}
/// Smtp Helo received
impl From<(StateHelo, SmtpHelo)> for StateHelo {
    fn from(tuple: (StateHelo, SmtpHelo)) -> Self {
        let (mut state, helo) = tuple;
        state.peer_helo = Some(helo);
        state
    }
}
/// Smtp Helo received
impl From<Connection> for StateHelo {
    fn from(connection: Connection) -> Self {
        Self {
            connection,
            peer_helo: None,
        }
    }
}
impl From<(StateHelo, SmtpMail, String)> for StateMail {
    fn from(tuple: (StateHelo, SmtpMail, String)) -> Self {
        let (
            StateHelo {
                connection,
                peer_helo,
            },
            mail,
            mailid,
        ) = tuple;
        Self {
            connection,
            peer_helo,
            mail,
            mailid,
            recipients: vec![],
        }
    }
}
impl From<(StateMail, SmtpPath)> for StateMail {
    fn from(tuple: (StateMail, SmtpPath)) -> Self {
        let (mut mail, rcpt) = tuple;
        mail.recipients.push(rcpt);
        mail
    }
}
impl<M: Sink<Vec<u8>, Error = Error> + Send + Sync + 'static> From<(StateMail, M)> for StateData {
    fn from(tuple: (StateMail, M)) -> Self {
        let (
            StateMail {
                connection,
                peer_helo,
                mailid,
                ..
            },
            sink,
        ) = tuple;
        StateData {
            connection,
            peer_helo,
            mailid,
            sink: Box::pin(sink),
        }
    }
}
