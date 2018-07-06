use bytes::Bytes;
use model::command::*;
use model::controll::*;
use model::response::SmtpReply;
use std::collections::VecDeque;
use std::net::SocketAddr;

#[derive(Clone)]
pub struct Session {
    state: State,
    name: String,
    peer: Option<SocketAddr>,
    local: Option<SocketAddr>,
    helo: Option<SmtpHelo>,
    mail: Option<SmtpMail>,
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
            rcpts: vec![],
            answers: vec![].into(),
        }
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
            PeerConnected(peer) => self.conn(peer),
            PeerShutdown(peer) => self.shut(peer),
            Invalid(_) => self.say_syntax_error(),
            Command(cmd) => self.cmd(cmd),
            DataChunk(data) => self.data(data),
            EscapeDot(_data) => self,
            FinalDot(_data) => self.data_end(),
        }
    }
    pub fn data_end(&mut self) -> &mut Self {
        trace!("watching data finishing up!");
        self.state = match self.state {
            State::New => State::New,
            _ => State::Helo,
        };
        self.rcpts.clear();
        self.mail = None;
        // leaving helo as is
        self.say_ok()
    }
    pub fn data(&mut self, _data: Bytes) -> &mut Self {
        trace!("watching data pass by!");
        self
    }
    pub fn conn(&mut self, peer: Option<SocketAddr>) -> &mut Self {
        self.state = State::New;
        self.peer = peer;
        self.rcpts.clear();
        self.mail = None;
        self.helo = None;
        self.say_ready()
    }
    pub fn shut(&mut self, _peer: Option<SocketAddr>) -> &mut Self {
        self.state = State::New;
        self.peer = None;
        self.rcpts.clear();
        self.mail = None;
        self.helo = None;
        self.say_shutdown()
    }
    pub fn cmd(&mut self, cmd: SmtpCommand) -> &mut Self {
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
    pub fn cmd_quit(&mut self) -> &mut Self {
        self.state = State::New;
        self.helo = None;
        self.mail = None;
        self.rcpts.clear();
        let name = self.name().into();
        self.say_reply(SmtpReply::ClosingConnectionInfo(name))
            .say_shutdown()
    }
    pub fn cmd_data(&mut self) -> &mut Self {
        if self.helo == None || self.mail == None || self.rcpts.len() == 0 {
            self.say_command_sequence_fail()
        } else {
            self.state = State::Data;
            self.say(ClientControll::AcceptData)
                .say_reply(SmtpReply::StartMailInputChallenge)
        }
    }
    pub fn cmd_rcpt(&mut self, path: SmtpPath) -> &mut Self {
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
    pub fn cmd_mail(&mut self, mail: SmtpMail) -> &mut Self {
        if self.state != State::Helo || self.helo == None {
            self.say_command_sequence_fail()
        } else {
            self.state = State::Mail;
            self.mail = Some(mail);
            self.say_ok()
        }
    }
    pub fn cmd_helo(&mut self, helo: SmtpHelo) -> &mut Self {
        // reset bufers
        self.rcpts.clear();
        self.mail = None;
        //set new state
        self.state = State::Helo;
        let remote = helo.name();
        self.helo = Some(helo);
        self.say_hi(remote)
    }
    pub fn cmd_rset(&mut self) -> &mut Self {
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
    pub fn cmd_noop(&mut self) -> &mut Self {
        self.say_ok()
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
    pub fn say(&mut self, c: ClientControll) -> &mut Self {
        self.answers.push_back(c);
        self
    }
    pub fn say_reply(&mut self, c: SmtpReply) -> &mut Self {
        self.say(ClientControll::Reply(c))
    }
    pub fn say_shutdown(&mut self) -> &mut Self {
        self.say(ClientControll::Shutdown)
    }
    pub fn say_ready(&mut self) -> &mut Self {
        let name = self.name().into();
        self.say_reply(SmtpReply::ServiceReadyInfo(name))
    }
    pub fn say_ok(&mut self) -> &mut Self {
        self.say_reply(SmtpReply::OkInfo)
    }
    pub fn say_hi(&mut self, remote: String) -> &mut Self {
        let local = self.name.clone();
        self.say_reply(SmtpReply::OkHeloInfo { local, remote })
    }
    pub fn say_command_sequence_fail(&mut self) -> &mut Self {
        self.say_reply(SmtpReply::CommandSequenceFailure)
    }
    pub fn say_syntax_error(&mut self) -> &mut Self {
        self.say_reply(SmtpReply::CommandSyntaxFailure)
    }
    pub fn say_not_implemented(&mut self) -> &mut Self {
        self.say_reply(SmtpReply::CommandNotImplementedFailure)
    }
}
