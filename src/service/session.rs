use bytes::Bytes;
use futures::prelude::*;
use model::command::*;
use model::controll::*;
use model::mail::*;
use model::response::SmtpReply;
use service::*;
use std::collections::VecDeque;
use std::net::SocketAddr;
use tokio::io;
use util::*;
use uuid::Uuid;

#[derive(Clone)]
pub struct StatefulSessionService<M> {
    mail_service: M,
}

impl<M> StatefulSessionService<M> {
    pub fn new(mail_service: M) -> Self {
        Self { mail_service }
    }
}

impl<M, H> SessionService for StatefulSessionService<M>
where
    M: MailService<MailDataWrite = H> + Clone,
{
    type Handler = SessionHandler<M, H>;
    fn start(&self) -> Self::Handler {
        let name = self.mail_service.name();
        SessionHandler::new(name, self.mail_service.clone())
    }
}

pub struct SessionHandler<M, H> {
    mail_service: M,
    mail_handler: Option<H>,
    session: Session,
}

impl<M, H> SessionHandler<M, H> {
    pub fn new(name: impl ToString, mail_service: M) -> Self {
        Self {
            mail_service,
            mail_handler: None,
            session: Session::new(name),
        }
    }
}

impl<M, H> Sink for SessionHandler<M, H>
where
    H: Sink<SinkError = io::Error>,
{
    type SinkItem = ServerControll;
    type SinkError = io::Error;
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        match self.mail_handler {
            None => {}
            Some(ref mut h) => match h.poll_complete() {
                nr @ Ok(Async::NotReady) => return nr,
                Err(e) => return Err(e),
                Ok(Async::Ready(())) => {}
            },
        };

        // TODO: Handle unresolved futures:
        //   - pending rcpt check
        Ok(Async::Ready(()))
    }
    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        match self.session.controll(item) {
            ControllResult::Ok => Ok(AsyncSink::Ready),
            ControllResult::Wait(item) => Ok(AsyncSink::NotReady(item)),
            ControllResult::Ended => Err(io::Error::new(
                io::ErrorKind::NotConnected,
                format!("The session is over!"),
            )),
        }
    }
}
impl<M, H> Stream for SessionHandler<M, H>
where
    M: MailService<MailDataWrite = H>,
    H: MailHandler,
    H: Sink<SinkItem = Bytes, SinkError = io::Error>,
{
    type Item = ClientControll;
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        // Pick an answer only in one place while passing the poll to the sink.
        // Whenever we later change the state we want to come back here right away
        // so we return a ClientControll::Noop and the consumer will come back for more.
        let answer = match self.session.get_answer() {
            None => match self.poll_complete() {
                Ok(Async::Ready(())) => match self.session.get_answer() {
                    None => return none(),
                    Some(answer) => answer,
                },
                Ok(Async::NotReady) => return pending(),
                Err(e) => return Err(e),
            },
            Some(answer) => answer,
        };

        match answer {
            SessionControll::Reply(reply) => ok(ClientControll::Reply(reply)),
            SessionControll::CheckRcpt(request) => {
                let result = self.mail_service.accept(request);
                self.session.rcpt_checked(result);
                // we did something, but want to be called again
                ok(ClientControll::Noop)
            }
            SessionControll::SendMail(envelope) => {
                if self.mail_handler.is_some() {
                    warn!("Asked to send mail, while another one is in progress. Bummer!");
                    // I'm going to be very strict here. This should not happen.
                    self.session.mail_sent(MailSendResult::Failed);
                    // we did something, but want to be called again
                    ok(ClientControll::Noop)
                } else {
                    let result = self.mail_service.mail(envelope);
                    match result {
                        None => {
                            self.session.mail_sent(MailSendResult::Rejected);
                            // we did something, but want to be called again
                            ok(ClientControll::Noop)
                        }
                        Some(h) => {
                            self.session.mail_sent(MailSendResult::Ok);
                            self.mail_handler = Some(h);
                            ok(ClientControll::AcceptData)
                        }
                    }
                }
            }
            SessionControll::Data(data) => {
                match self.mail_handler {
                    None => {
                        warn!("Asked to write mail data without a handler. Bummer!");
                        self.session.error_sending_data();
                        // we did something, but want to be called again
                        ok(ClientControll::Noop)
                    }
                    Some(ref mut h) => {
                        match h.start_send(data) {
                            Ok(AsyncSink::Ready) => {
                                /*Yay! Good stuff... */
                                ok(ClientControll::Noop)
                            }
                            Ok(AsyncSink::NotReady(data)) => {
                                /*Push back from the sink!*/
                                self.session.push_back(SessionControll::Data(data));
                                ok(ClientControll::Noop)
                            }
                            Err(e) => {
                                warn!("Mail data write error. {:?}", e);
                                self.session.error_sending_data();
                                ok(ClientControll::Noop)
                            }
                        }
                    }
                }
            }
            SessionControll::QueueMail => {
                match self.mail_handler.take() {
                    None => {
                        warn!("Asked to queue mail without a handler. Bummer!");
                        self.session.queued(QueueResult::Failed);
                        // we did something, but want to be called again
                        ok(ClientControll::Noop)
                    }
                    Some(h) => {
                        let result = h.queue();
                        self.session.queued(result);
                        // we did something, but want to be called again
                        ok(ClientControll::Noop)
                    }
                }
            }
            SessionControll::EndOfSession => ok(ClientControll::Shutdown),
        }
    }
}

pub enum ControllResult<T> {
    Wait(T),
    Ok,
    Ended,
}

#[derive(Clone)]
pub enum SessionControll {
    QueueMail,
    SendMail(Envelope),
    CheckRcpt(AcceptRecipientRequest),
    EndOfSession,
    Reply(SmtpReply),
    Data(Bytes),
}
pub enum MailSendResult {
    Ok,
    Rejected,
    Failed,
}

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
    RcptCheck,
    Rcpt,
    Data,
    Queue,
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
        if self.answers.len() != 0 {
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
        };
        ControllResult::Ok
    }
    pub fn error_sending_data(&mut self) -> &mut Self {
        // Should never hapen. If it does, run.
        warn!("error_sending_data() called in state {:?}", self.state);
        self.end()
    }
    pub fn queued(&mut self, result: QueueResult) -> &mut Self {
        if self.state == State::Queue {
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
    pub fn mail_sent(&mut self, result: MailSendResult) -> &mut Self {
        if self.state == State::Data {
            match result {
                MailSendResult::Ok => self.say_reply(SmtpReply::StartMailInputChallenge),
                MailSendResult::Failed => self.say_mail_queue_failed(),
                MailSendResult::Rejected => self.say_mail_not_accepted(),
            }
        } else {
            // Should never hapen. If it does, run.
            warn!("mail_sent() called in state {:?}", self.state);
            self.end()
        }
    }
    pub fn rcpt_checked(&mut self, result: AcceptRecipientResult) -> &mut Self {
        if self.state == State::RcptCheck {
            self.state = State::Rcpt;
            use model::mail::AcceptRecipientResult::*;
            match result {
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
        if self.state == State::Data {
            self.state = State::Queue;
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
        if self.state == State::Data {
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
            self.state = State::Data;
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
            self.state = State::RcptCheck;
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
