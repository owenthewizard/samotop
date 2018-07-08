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
    answers: VecDeque<ClientControll>,
}

#[derive(Clone, PartialEq, Eq)]
enum State {
    New,
    Helo,
    Mail,
    Rcpt,
    Data,
}

impl Session {
    pub fn new() -> Self {
        Self {
            state: State::New,
            name: "Samotop".into(),
            peer: None,
            local: None,
            helo: None,
            mail: None,
            mailid: Uuid::new_v4(),
            rcpts: vec![],
            answers: vec![].into(),
        }
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn helo(&self) -> Option<&SmtpHelo> {
        self.helo.as_ref()
    }
    pub fn mail(&self) -> Option<&SmtpMail> {
        self.mail.as_ref()
    }
    pub fn peer(&self) -> Option<&SocketAddr> {
        self.peer.as_ref()
    }
    pub fn rcpts(&self) -> impl Iterator<Item = &SmtpPath> {
        self.rcpts.iter()
    }
    pub fn set_name(&mut self, name: impl ToString) {
        self.name = name.to_string();
    }
    pub fn answer(&mut self) -> Option<ClientControll> {
        self.answers.pop_front()
    }
    pub fn controll(&mut self, ctrl: ServerControll) -> &mut Self {
        use model::controll::ServerControll::*;
        match ctrl {
            PeerConnected { local, peer } => self.conn(local, peer),
            PeerShutdown => self.shut(),
            Invalid(_) => self.say_syntax_error(),
            Command(cmd) => self.cmd(cmd),
            DataChunk(data) => self.data(data),
            EscapeDot(_data) => self,
            FinalDot(_data) => self.data_end(),
        }
    }
    pub fn cancel(&mut self) -> &mut Self {
        // reset bufers
        self.rcpts.clear();
        self.mail = None;
        // leaving helo as is
        //set new state
        self.state = match self.state {
            State::New => State::New,
            _ => State::Helo,
        };
        // also clear any pending answers
        self.answers.clear();
        self
    }

    /// Returns a snapshot of the current mail session buffers.
    pub fn extract_envelope(&self) -> Envelope {
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
    /// Returns a snapshot of the current mail session buffers.
    pub fn extract_rcpt(&self, rcpt: &SmtpPath) -> AcceptRecipientRequest {
        AcceptRecipientRequest {
            name: self.name.clone(),
            local: self.local.clone(),
            peer: self.peer.clone(),
            helo: self.helo.clone(),
            mail: self.mail.clone(),
            id: self.mailid.to_string(),
            rcpt: rcpt.clone(),
        }
    }

    fn data_end(&mut self) -> &mut Self {
        trace!("watching data finishing up!");
        self.state = match self.state {
            State::New => State::New,
            _ => State::Helo,
        };
        self.rcpts.clear();
        self.mail = None;
        // leaving helo as is
        // TODO: need a better solution here.
        //  - who is responsibile for the answers?
        //self.say_ok()
        let id = self.mailid.to_string();
        self.say(ClientControll::QueueMail)
            .say_reply(SmtpReply::OkMessageInfo(format!("Queued as {}", id)))
    }
    fn data(&mut self, _data: Bytes) -> &mut Self {
        trace!("watching data pass by!");
        self
    }
    fn conn(&mut self, local: Option<SocketAddr>, peer: Option<SocketAddr>) -> &mut Self {
        self.state = State::New;
        self.local = local;
        self.peer = peer;
        self.rcpts.clear();
        self.mail = None;
        self.helo = None;
        self.say_ready()
    }
    fn shut(&mut self) -> &mut Self {
        self.state = State::New;
        self.peer = None;
        self.rcpts.clear();
        self.mail = None;
        self.helo = None;
        self.say_shutdown()
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
        self.state = State::New;
        self.helo = None;
        self.mail = None;
        self.rcpts.clear();
        let name = self.name().into();
        self.say_reply(SmtpReply::ClosingConnectionInfo(name))
            .say_shutdown()
    }
    fn cmd_data(&mut self) -> &mut Self {
        if self.helo == None || self.mail == None || self.rcpts.len() == 0 {
            self.say_command_sequence_fail()
        } else {
            self.state = State::Data;
            self.say(ClientControll::AcceptData)
                .say_reply(SmtpReply::StartMailInputChallenge)
        }
    }
    fn cmd_rcpt(&mut self, path: SmtpPath) -> &mut Self {
        if (self.state != State::Mail && self.state != State::Rcpt)
            || self.helo == None
            || self.mail == None
        {
            self.say_command_sequence_fail()
        } else {
            self.state = State::Rcpt;
            self.rcpts.push(path);
            self.say_ok()
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
        // new mail ID
        self.mailid = Uuid::new_v4();
        // reset bufers
        self.rcpts.clear();
        self.mail = None;
        //set new state
        self.state = State::Helo;
        let remote = helo.name();
        self.helo = Some(helo);
        self.say_hi(remote)
    }
    fn cmd_rset(&mut self) -> &mut Self {
        // reset bufers
        self.rcpts.clear();
        self.mail = None;
        // leaving helo as is
        //set new state
        self.state = match self.state {
            State::New => State::New,
            _ => State::Helo,
        };
        self.say_ok()
    }
    fn cmd_noop(&mut self) -> &mut Self {
        self.say_ok()
    }
    fn say(&mut self, c: ClientControll) -> &mut Self {
        self.answers.push_back(c);
        self
    }
    fn say_reply(&mut self, c: SmtpReply) -> &mut Self {
        self.say(ClientControll::Reply(c))
    }
    fn say_shutdown(&mut self) -> &mut Self {
        self.say(ClientControll::Shutdown)
    }
    fn say_ready(&mut self) -> &mut Self {
        let name = self.name().into();
        self.say_reply(SmtpReply::ServiceReadyInfo(name))
    }
    fn say_ok(&mut self) -> &mut Self {
        self.say_reply(SmtpReply::OkInfo)
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
}
