use bytes::Bytes;
use model::command::*;
use model::controll::*;
use model::mail::*;
use model::response::SmtpReply;
use std::collections::VecDeque;
use std::net::SocketAddr;
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
    answers: VecDeque<SessionControll>,
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
        }
    }
    pub fn get_answer(&mut self) -> Option<SessionControll> {
        self.answers.pop_front()
    }
    pub fn push_back(&mut self, ctrl: SessionControll) -> &mut Self {
        self.answers.push_front(ctrl);
        self
    }
    pub fn controll(&mut self, ctrl: ServerControll) -> ControllResult<ServerControll> {
        if self.answers.len() != 0
            || self.state == State::WaitForRcptCheck
            || self.state == State::WaitForSendMail
            || self.state == State::WaitForQueue
        {
            trace!("pushing back");
            // We're waiting for something or we've got some backlog to clear. Push back!
            return ControllResult::Wait(ctrl);
        }

        if self.state == State::End {
            return ControllResult::Ended;
        }

        use model::controll::ServerControll::*;
        match ctrl {
            PeerConnected { local, peer } => self.conn(local, peer),
            PeerShutdown => self.end(),
            Invalid(_) => self.say_syntax_error(),
            Command(cmd) => self.cmd(cmd),
            DataChunk(data) => self.data_chunk(data),
            EscapeDot(_data) => self,
            FinalDot(_data) => self.data_end(),
            ConfirmSwitchToData => self.confirm_data(),
        };
        ControllResult::Ok
    }
    pub fn error_sending_data(&mut self) -> &mut Self {
        // Should never hapen. If it does, run.
        warn!("error_sending_data() called in state {:?}", self.state);
        self.end()
    }
    pub fn mail_queued(&mut self, result: QueueResult) -> &mut Self {
        if self.state == State::WaitForQueue {
            match result {
                QueueResult::QueuedWithId(id) => {
                    self.rst().say_ok_info(format!("Queued as {}", id))
                }
                QueueResult::Failed => self.rst().say_mail_queue_failed(),
                QueueResult::Refused => self.rst().say_mail_not_accepted(),
            }
        } else {
            // Should never hapen. If it does, run.
            warn!("queued() called in state {:?}", self.state);
            self.end()
        }
    }
    pub fn mail_sending(&mut self, result: MailSendResult) -> &mut Self {
        if self.state == State::WaitForSendMail {
            match result {
                MailSendResult::Ok => {
                    self.state = State::DataStreaming;
                    self.say(SessionControll::AcceptMailData(true))
                        .say_reply(SmtpReply::StartMailInputChallenge)
                }
                MailSendResult::Failed => self
                    .rst()
                    .say(SessionControll::AcceptMailData(false))
                    .say_mail_queue_failed(),
                MailSendResult::Rejected => self
                    .rst()
                    .say(SessionControll::AcceptMailData(false))
                    .say_mail_not_accepted(),
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
            use model::mail::AcceptRecipientResult::*;
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
    fn confirm_data(&mut self) -> &mut Self {
        if self.state == State::DataStreaming {
            self.say(SessionControll::AcceptMailData(true))
        } else {
            warn!("rejecting data?");
            self.say(SessionControll::AcceptMailData(false))
        }
    }
    fn data_end(&mut self) -> &mut Self {
        if self.state == State::DataStreaming {
            self.state = State::WaitForQueue;
            // confirmation or disaproval SmtpReply is sent after
            // self.queued(..)
            self.say(SessionControll::QueueMail)
        } else {
            // Should never hapen. If it does, run.
            warn!("data_end() called in state {:?}", self.state);
            self.end()
        }
    }
    fn data_chunk(&mut self, data: Bytes) -> &mut Self {
        if self.state == State::DataStreaming {
            self.say(SessionControll::Data(data))
        } else {
            // Should never hapen. If it does, run.
            warn!("data_chunk() called in state {:?}", self.state);
            self.end()
        }
    }
    fn conn(&mut self, local: Option<SocketAddr>, peer: Option<SocketAddr>) -> &mut Self {
        self.rst_to_new();
        self.local = local;
        self.peer = peer;
        self.say_ready()
    }
    fn end(&mut self) -> &mut Self {
        self.rst_to_new().say_end_of_session().state = State::End;
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
        use model::command::SmtpCommand::*;
        match cmd {
            Helo(from) => self.cmd_helo(from),
            Mail(mail) => self.cmd_mail(mail),
            Rcpt(path) => self.cmd_rcpt(path),
            Data => self.cmd_data(),
            Quit => self.cmd_quit(),
            Rset => self.cmd_rset(),
            Noop(_) => self.cmd_noop(),
            _ => self.say_not_implemented(),
        }
    }
    fn cmd_quit(&mut self) -> &mut Self {
        let name = self.name.clone();
        self.rst_to_new()
            .say_reply(SmtpReply::ClosingConnectionInfo(name))
            .say_end_of_session()
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
            self.say(SessionControll::SendMail(envelope))
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
            self.say(SessionControll::CheckRcpt(request))
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
        self.helo = Some(helo);
        self.state = State::Helo;
        self.say_hi(remote)
    }
    fn cmd_rset(&mut self) -> &mut Self {
        self.rst().say_ok()
    }
    fn cmd_noop(&mut self) -> &mut Self {
        self.say_ok()
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

    fn say(&mut self, c: SessionControll) -> &mut Self {
        self.answers.push_back(c);
        self
    }
    fn say_reply(&mut self, c: SmtpReply) -> &mut Self {
        self.say(SessionControll::Reply(c))
    }
    fn say_end_of_session(&mut self) -> &mut Self {
        self.say(SessionControll::EndOfSession)
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
    fn say_hi(&mut self, remote: String) -> &mut Self {
        let local = self.name.clone();
        self.say_reply(SmtpReply::OkHeloInfo { local, remote })
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
    fn say_mail_not_accepted(&mut self) -> &mut Self {
        self.say_reply(SmtpReply::MailboxNotAvailableFailure)
    }
    fn say_mail_queue_failed(&mut self) -> &mut Self {
        self.say_reply(SmtpReply::MailboxNotAvailableError)
    }
}

pub enum ControllResult<T> {
    Wait(T),
    Ok,
    Ended,
}

#[derive(Clone, Debug)]
pub enum SessionControll {
    QueueMail,
    SendMail(Envelope),
    CheckRcpt(AcceptRecipientRequest),
    EndOfSession,
    AcceptMailData(bool),
    Reply(SmtpReply),
    Data(Bytes),
    Fail,
}

pub enum MailSendResult {
    Ok,
    Rejected,
    Failed,
}
