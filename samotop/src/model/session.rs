use crate::model::io::*;
use crate::model::mail::*;
use crate::model::smtp::*;
use bytes::Bytes;
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::result::Result;
use uuid::Uuid;

#[derive(Clone)]
pub struct Session {
    state: State,
    name: String,
    peer: Option<SocketAddr>,
    local: Option<SocketAddr>,
    helo: Option<SmtpHelo>,
    mail: Option<SmtpMail>,
    mailid: Uuid,
    rcpts: Vec<SmtpPath>,
    answers: VecDeque<SessionControl>,
    extensions: ExtensionSet,
}

#[derive(Clone, Debug)]
pub enum SessionControl {
    QueueMail,
    SendMail(Envelope),
    CheckRcpt(AcceptRecipientRequest),
    EndOfSession,
    StartData(SmtpReply),
    StartTls(SmtpReply),
    Reply(SmtpReply),
    Data(Bytes),
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum State {
    New,
    Helo,
    Mail,
    WaitForRcptCheck,
    Rcpt,
    WaitForSendMail,
    DataStreaming,
    WaitForQueue,
    End,
}

impl Session {
    pub fn new(name: impl ToString) -> Self {
        Self {
            state: State::New,
            name: name.to_string(),
            peer: None,
            local: None,
            helo: None,
            mail: None,
            mailid: Uuid::new_v4(),
            rcpts: vec![],
            answers: vec![].into(),
            extensions: ExtensionSet::new(),
        }
    }
    pub fn get_answer(&mut self) -> Option<SessionControl> {
        self.answers.pop_front()
    }
    pub fn push_back(&mut self, ctrl: SessionControl) -> &mut Self {
        self.answers.push_front(ctrl);
        self
    }
    /// Can the machine take more controls?
    pub fn is_ready(&self) -> bool {
        self.state != State::WaitForRcptCheck
            && self.state != State::WaitForSendMail
            && self.state != State::WaitForQueue
    }
    pub fn is_closed(&self) -> bool {
        self.state == State::End
    }
    pub fn control(&mut self, ctrl: ReadControl) -> Result<(), ControlResult> {
        if !self.is_ready() {
            trace!("pushing back");
            // We're waiting for something or we've got some backlog to clear. Push back!
            return Err(ControlResult::Wait);
        }

        if self.is_closed() {
            return Err(ControlResult::Ended);
        }

        // Todo: handle delayed banner
        match ctrl {
            ReadControl::PeerConnected(conn) => self.conn(conn),
            ReadControl::PeerShutdown => self.end(),
            ReadControl::Raw(_) => self.say_syntax_error(),
            ReadControl::Command(cmd) => self.cmd(cmd),
            ReadControl::MailDataChunk(data) => self.data_chunk(data),
            ReadControl::EscapeDot(_data) => self,
            ReadControl::EndOfMailData(_data) => self.data_end(),
            ReadControl::Empty(_data) => self,
        };
        Ok(())
    }
    pub fn error_sending_data(&mut self) -> &mut Self {
        // Should never hapen. If it does, run.
        warn!("error_sending_data() called in state {:?}", self.state);
        self.end()
    }
    pub fn mail_queued(&mut self, result: QueueResult) -> &mut Self {
        if self.state == State::WaitForQueue {
            match result {
                Ok(()) => self.say_mail_queued().rst(),
                Err(QueueError::Failed) => self.say_mail_queue_failed_temporarily().rst(),
                Err(QueueError::Refused) => self.say_mail_queue_refused().rst(),
            }
        } else {
            // Should never hapen. If it does, run.
            warn!("queued() called in state {:?}", self.state);
            self.end()
        }
    }
    pub fn mail_sending(&mut self, result: QueueResult) -> &mut Self {
        if self.state == State::WaitForSendMail {
            match result {
                Ok(()) => {
                    self.state = State::DataStreaming;
                    self.say(SessionControl::StartData(
                        SmtpReply::StartMailInputChallenge,
                    ))
                }
                Err(QueueError::Failed) => self.say_mail_queue_failed_temporarily().rst(),
                Err(QueueError::Refused) => self.say_mail_queue_refused().rst(),
            }
        } else {
            // Should never hapen. If it does, run.
            warn!("mail_sent() called in state {:?}", self.state);
            self.end()
        }
    }
    pub fn rcpt_checked(&mut self, result: AcceptRecipientResult) -> &mut Self {
        if self.state == State::WaitForRcptCheck {
            self.state = State::Rcpt;
            use crate::model::mail::AcceptRecipientResult::*;
            match result {
                Failed => self.say_reply(SmtpReply::ProcesingError),
                Rejected => self.say_recipient_not_accepted(),
                RejectedWithNewPath(path) => self.say_recipient_not_local(path),
                Accepted(path) => {
                    self.rcpts.push(path);
                    self.say_ok()
                }
                AcceptedWithNewPath(path) => {
                    self.rcpts.push(path.clone());
                    self.say_ok_recipient_not_local(path)
                }
            }
        } else {
            // Should never hapen. If it does, run.
            warn!("rcpt_checked() called in state {:?}", self.state);
            self.end()
        }
    }
    fn data_end(&mut self) -> &mut Self {
        if self.state == State::DataStreaming {
            self.state = State::WaitForQueue;
            // confirmation or disaproval SmtpReply is sent after
            // self.queued(..)
            self.say(SessionControl::QueueMail)
        } else {
            // Should never hapen. If it does, run.
            warn!("data_end() called in state {:?}", self.state);
            self.end()
        }
    }
    fn data_chunk(&mut self, data: Bytes) -> &mut Self {
        if self.state == State::DataStreaming {
            self.say(SessionControl::Data(data))
        } else {
            // Should never hapen. If it does, run.
            warn!("data_chunk() called in state {:?}", self.state);
            self.end()
        }
    }
    fn conn(&mut self, conn: Connection) -> &mut Self {
        self.rst_to_new();
        self.local = conn.local_addr();
        self.peer = conn.peer_addr();
        self.extensions = conn.extensions().clone();
        self.say_ready()
    }
    fn end(&mut self) -> &mut Self {
        self.say_end_of_session().rst_to_new().state = State::End;
        self
    }
    fn rst_to_new(&mut self) -> &mut Self {
        self.state = State::New;
        self.helo = None;
        self.rst()
    }
    fn rst(&mut self) -> &mut Self {
        if self.helo == None {
            self.state = State::New;
        } else {
            self.state = State::Helo;
        }
        self.mail = None;
        self.rcpts.clear();
        self.mailid = Uuid::new_v4();
        self
    }
    fn cmd(&mut self, cmd: SmtpCommand) -> &mut Self {
        use SmtpCommand::*;
        match cmd {
            Helo(from) => self.cmd_helo(from),
            Mail(mail) => self.cmd_mail(mail),
            Rcpt(path) => self.cmd_rcpt(path),
            Data => self.cmd_data(),
            Quit => self.cmd_quit(),
            Rset => self.cmd_rset(),
            Noop(_) => self.cmd_noop(),
            StartTls => self.cmd_starttls(),
            _ => self.say_not_implemented(),
        }
    }
    fn cmd_quit(&mut self) -> &mut Self {
        let name = self.name.clone();
        self.say_reply(SmtpReply::ClosingConnectionInfo(name))
            .say_end_of_session()
            .rst_to_new()
    }
    fn cmd_data(&mut self) -> &mut Self {
        if self.state != State::Rcpt
            || self.helo == None
            || self.mail == None
            || self.rcpts.len() == 0
        {
            self.say_command_sequence_fail()
        } else {
            self.state = State::WaitForSendMail;
            let envelope = self.make_envelope();
            self.say(SessionControl::SendMail(envelope))
        }
    }
    fn cmd_rcpt(&mut self, rcpt: SmtpPath) -> &mut Self {
        if (self.state != State::Mail && self.state != State::Rcpt)
            || self.helo == None
            || self.mail == None
        {
            self.say_command_sequence_fail()
        } else {
            self.state = State::WaitForRcptCheck;
            let request = self.make_rcpt_request(rcpt);
            self.say(SessionControl::CheckRcpt(request))
        }
    }
    fn cmd_mail(&mut self, mail: SmtpMail) -> &mut Self {
        if self.state != State::Helo || self.helo == None {
            self.say_command_sequence_fail()
        } else {
            self.state = State::Mail;
            self.mail = Some(mail);
            self.say_ok()
        }
    }
    fn cmd_helo(&mut self, helo: SmtpHelo) -> &mut Self {
        self.rst_to_new();
        let remote = helo.name();
        let extended = helo.is_extended();
        self.helo = Some(helo);
        self.state = State::Helo;
        match extended {
            false => self.say_helo(remote),
            true => self.say_ehlo(remote),
        }
    }
    fn cmd_rset(&mut self) -> &mut Self {
        self.say_ok().rst()
    }
    fn cmd_noop(&mut self) -> &mut Self {
        self.say_ok()
    }
    fn cmd_starttls(&mut self) -> &mut Self {
        if self.state != State::Helo || self.helo == None {
            self.say_command_sequence_fail()
        } else {
            let name = self.name.clone();
            // you cannot STARTTLS twice so we only advertise it before first use
            if self.extensions.disable(SmtpExtension::STARTTLS.code) {
                // TODO: better message response
                self.say(SessionControl::StartTls(SmtpReply::ServiceReadyInfo(name)))
            } else {
                self.say_not_implemented()
            }
        }
    }

    /// Returns a snapshot of the current mail session buffers.
    fn make_rcpt_request(&self, rcpt: SmtpPath) -> AcceptRecipientRequest {
        AcceptRecipientRequest {
            name: self.name.clone(),
            local: self.local.clone(),
            peer: self.peer.clone(),
            helo: self.helo.clone(),
            mail: self.mail.clone(),
            id: self.mailid.to_string(),
            rcpt: rcpt,
        }
    }

    /// Returns a snapshot of the current mail session buffers.
    fn make_envelope(&self) -> Envelope {
        Envelope {
            name: self.name.clone(),
            local: self.local.clone(),
            peer: self.peer.clone(),
            helo: self.helo.clone(),
            mail: self.mail.clone(),
            rcpts: self.rcpts.clone(),
            id: self.mailid.to_string(),
        }
    }

    fn say(&mut self, c: SessionControl) -> &mut Self {
        self.answers.push_back(c);
        self
    }
    fn say_reply(&mut self, c: SmtpReply) -> &mut Self {
        self.say(SessionControl::Reply(c))
    }
    fn say_end_of_session(&mut self) -> &mut Self {
        self.say(SessionControl::EndOfSession)
    }
    fn say_ready(&mut self) -> &mut Self {
        let name = self.name.clone();
        self.say_reply(SmtpReply::ServiceReadyInfo(name))
    }
    fn say_ok(&mut self) -> &mut Self {
        self.say_reply(SmtpReply::OkInfo)
    }
    fn say_ok_info(&mut self, info: String) -> &mut Self {
        self.say_reply(SmtpReply::OkMessageInfo(info))
    }
    fn say_helo(&mut self, remote: String) -> &mut Self {
        let local = self.name.clone();
        self.say_reply(SmtpReply::OkHeloInfo { local, remote })
    }
    fn say_ehlo(&mut self, remote: String) -> &mut Self {
        let local = self.name.clone();
        let extensions = self.extensions.iter().map(Clone::clone).collect();
        self.say_reply(SmtpReply::OkEhloInfo {
            local,
            remote,
            extensions,
        })
    }
    fn say_command_sequence_fail(&mut self) -> &mut Self {
        self.say_reply(SmtpReply::CommandSequenceFailure)
    }
    fn say_syntax_error(&mut self) -> &mut Self {
        self.say_reply(SmtpReply::CommandSyntaxFailure)
    }
    fn say_not_implemented(&mut self) -> &mut Self {
        self.say_reply(SmtpReply::CommandNotImplementedFailure)
    }
    fn say_recipient_not_accepted(&mut self) -> &mut Self {
        self.say_reply(SmtpReply::MailboxNotAvailableFailure)
    }
    fn say_recipient_not_local(&mut self, path: SmtpPath) -> &mut Self {
        self.say_reply(SmtpReply::UserNotLocalFailure(format!("{}", path)))
    }
    fn say_ok_recipient_not_local(&mut self, path: SmtpPath) -> &mut Self {
        self.say_reply(SmtpReply::UserNotLocalInfo(format!("{}", path)))
    }
    fn say_mail_queued(&mut self) -> &mut Self {
        let info = format!("Queued as {}", self.mailid);
        self.say_ok_info(info)
    }
    fn say_mail_queue_refused(&mut self) -> &mut Self {
        self.say_reply(SmtpReply::MailboxNotAvailableFailure)
    }
    fn say_mail_queue_failed_temporarily(&mut self) -> &mut Self {
        self.say_reply(SmtpReply::MailboxNotAvailableError)
    }
}

pub enum ControlResult {
    Wait,
    Ended,
}
