use crate::common::*;
use crate::model::mail::*;
use crate::model::session::*;
use crate::model::smtp::*;
use crate::service::mail::MailService;
use crate::service::session::stateful::{SessionHandler, SessionState};

#[derive(Clone)]
pub struct BasicSessionHandler<S> {
    service: S,
}

impl<S> From<S> for BasicSessionHandler<S>
where
    S: MailService,
{
    fn from(service: S) -> Self {
        Self { service }
    }
}

impl<S: MailService> SessionHandler for BasicSessionHandler<S> {
    type Data = Buffers;
    fn pop(&self, data: &mut Self::Data) -> Option<WriteControl> {
        data.answers.pop_front()
    }
    fn handle(&self, data: Self::Data, control: ReadControl) -> SessionState<Self::Data> {
        match control {
            ReadControl::PeerConnected(conn) => self.handle_conn(data, conn),
            ReadControl::PeerShutdown => self.handle_shutdown(data),
            ReadControl::Raw(_) => self.handle_raw(data),
            ReadControl::Command(cmd, _) => self.handle_cmd(data, cmd),
            ReadControl::MailDataChunk(bytes) => self.handle_data_chunk(data, bytes),
            ReadControl::EndOfMailData(_) => self.handle_data_end(data),
            ReadControl::Empty(_) => SessionState::Ready(data),
            ReadControl::EscapeDot(_) => SessionState::Ready(data),
        }
    }
}

impl<S> BasicSessionHandler<S> {
    pub fn new(service: S) -> Self {
        Self { service }
    }
}

impl<S: MailService> BasicSessionHandler<S> {
    pub fn handle_cmd(&self, data: Buffers, cmd: SmtpCommand) -> SessionState<Buffers> {
        use SmtpCommand::*;
        match cmd {
            Helo(from) => self.cmd_helo(data, from),
            Mail(mail) => self.cmd_mail(data, mail),
            Rcpt(path) => self.cmd_rcpt(data, path),
            Data => self.cmd_data(data),
            Quit => self.cmd_quit(data),
            Rset => self.cmd_rset(data),
            Noop(_) => self.cmd_noop(data),
            StartTls => self.cmd_starttls(data),
            Expn(_) => self.cmd_unknown(data),
            Vrfy(_) => self.cmd_unknown(data),
            Help(_) => self.cmd_unknown(data),
            Turn => self.cmd_unknown(data),
            Other(_, _) => self.cmd_unknown(data),
        }
    }
    pub fn handle_data_chunk(&self, mut data: Buffers, bytes: Vec<u8>) -> SessionState<Buffers> {
        match std::mem::replace(&mut data.state, State::Closed) {
            State::Data(state) => {
                let StateData {
                    mut sink,
                    mailid,
                    session,
                } = state;
                let fut = async move {
                    match sink.write_all(&bytes[..]).await {
                        Ok(()) => {
                            data.state = State::Data(StateData {
                                sink,
                                mailid,
                                session,
                            });
                            data
                        }
                        Err(e) => {
                            warn!("Failed to write mail data for {} - {}", mailid, e);
                            data.state = State::Connected(session);
                            // CheckMe: following this reset, we are not sending any response yet. handle_data_end should do that.
                            data
                        }
                    }
                };
                SessionState::Pending(Box::pin(fut))
            }
            other => {
                // CheckMe: silence. handle_data_end should respond with error.
                data.state = other;
                SessionState::Ready(data)
            }
        }
    }
    pub fn handle_data_end(&self, mut data: Buffers) -> SessionState<Buffers> {
        match std::mem::replace(&mut data.state, State::Closed) {
            State::Data(state) => {
                let StateData {
                    mut sink,
                    mailid,
                    session,
                } = state;
                let fut = async move {
                    match sink.close().await {
                        Ok(()) => {
                            data.say_mail_queued(mailid.as_str());
                        }
                        Err(e) => {
                            warn!("Failed to finish mail data for {} - {}", mailid, e);
                            data.say_mail_queue_failed_temporarily();
                        }
                    }
                    data.state = State::Connected(session);
                    data
                };
                SessionState::Pending(Box::pin(fut))
            }
            other => {
                // CheckMe: silence. handle_data_end should respond with error.
                data.state = other;
                SessionState::Ready(data)
            }
        }
    }
    pub fn handle_raw(&self, mut data: Buffers) -> SessionState<Buffers> {
        data.say_reply(SmtpReply::CommandSyntaxFailure);
        SessionState::Ready(data)
    }
    pub fn handle_conn(&self, mut data: Buffers, mut sess: SessionInfo) -> SessionState<Buffers> {
        self.service.prepare_session(&mut sess);
        data.say_service_ready(sess.service_name.as_str());
        data.state = State::Connected(sess);
        SessionState::Ready(data)
    }
    pub fn handle_shutdown(&self, mut data: Buffers) -> SessionState<Buffers> {
        data.state = State::Closed;
        SessionState::Ready(data)
    }

    fn cmd_unknown(&self, mut data: Buffers) -> SessionState<Buffers> {
        data.say_not_implemented();
        SessionState::Ready(data)
    }
    fn cmd_helo(&self, mut data: Buffers, helo: SmtpHelo) -> SessionState<Buffers> {
        let remote = helo.name();
        let extended = helo.is_extended();
        let get_extensions =
            |conn: &SessionInfo| conn.extensions.iter().map(str::to_owned).collect();
        let respond = |data: &mut Buffers, name, exts| match extended {
            false => {
                data.say_helo(name, remote);
            }
            true => {
                data.say_ehlo(name, exts, remote);
            }
        };
        data.state = match std::mem::replace(&mut data.state, State::Closed) {
            current @ State::New | current @ State::Closed => {
                data.say_command_sequence_fail();
                current
            }
            State::Connected(mut session) => {
                let exts = get_extensions(&session);
                session.smtp_helo = Some(helo);
                respond(&mut data, session.service_name.as_str(), exts);
                State::Connected(session)
            }
            State::Mail(mut envelope) => {
                let exts = get_extensions(&envelope.session);
                envelope.session.smtp_helo = Some(helo);
                respond(&mut data, envelope.session.service_name.as_str(), exts);
                State::Connected(envelope.session)
            }
            State::Data(mut state) => {
                let exts = get_extensions(&state.session);
                state.session.smtp_helo = Some(helo);
                respond(&mut data, state.session.service_name.as_str(), exts);
                State::Connected(state.session)
            }
        };
        SessionState::Ready(data)
    }
    fn cmd_quit(&self, mut data: Buffers) -> SessionState<Buffers> {
        let name = match data.state {
            // should not happen
            State::New | State::Closed => "samotop",
            State::Connected(ref session) => session.service_name.as_str(),
            State::Mail(ref envelope) => envelope.session.service_name.as_str(),
            State::Data(ref state) => state.session.service_name.as_str(),
        };
        let name = name.to_owned();
        data.say_shutdown_ok(name);
        data.state = State::Closed;
        SessionState::Ready(data)
    }
    fn cmd_mail(&self, mut data: Buffers, mail: SmtpMail) -> SessionState<Buffers> {
        match std::mem::replace(&mut data.state, State::Closed) {
            State::Connected(session) if session.smtp_helo.is_some() => {
                let request = StartMailRequest {
                    session: session.clone(),
                    id: String::new(),
                    mail: Some(mail.clone()),
                    rcpts: vec![],
                };
                let fut = self.service.start_mail(request).map(move |res| {
                    use StartMailResult as R;
                    data.state = match res {
                        R::Failed(StartMailFailure::TerminateSession, description) => {
                            data.say_shutdown_err(description);
                            State::Closed
                        }
                        R::Failed(failure, description) => {
                            data.say_mail_failed(failure, description);
                            State::Connected(session)
                        }
                        R::Accepted(envelope) => {
                            data.say_ok();
                            State::Mail(envelope)
                        }
                    };
                    data
                });

                SessionState::Pending(Box::pin(fut))
            }
            other @ State::Connected(_)
            | other @ State::Data(_)
            | other @ State::Mail(_)
            | other @ State::New
            | other @ State::Closed => {
                data.state = other;
                data.say_command_sequence_fail();
                SessionState::Ready(data)
            }
        }
    }
    fn cmd_rcpt(&self, mut data: Buffers, rcpt: SmtpPath) -> SessionState<Buffers> {
        match std::mem::replace(&mut data.state, State::Closed) {
            State::Mail(transaction) => {
                let request = AddRecipientRequest { transaction, rcpt };
                let fut = self.service.add_recipient(request).map(move |res| {
                    data.state = match res {
                        AddRecipientResult::TerminateSession(description) => {
                            data.say_shutdown_err(description);
                            State::Closed
                        }
                        AddRecipientResult::Failed(transaction, failure, description) => {
                            data.say_rcpt_failed(failure, description);
                            State::Mail(transaction)
                        }
                        AddRecipientResult::Accepted(transaction) => {
                            data.say_ok();
                            State::Mail(transaction)
                        }
                        AddRecipientResult::AcceptedWithNewPath(transaction, path) => {
                            data.say_ok_recipient_not_local(path);
                            State::Mail(transaction)
                        }
                    };
                    data
                });

                SessionState::Pending(Box::pin(fut))
            }
            other @ State::Connected(_)
            | other @ State::Data(_)
            | other @ State::New
            | other @ State::Closed => {
                data.state = other;
                data.say_command_sequence_fail();
                SessionState::Ready(data)
            }
        }
    }
    fn cmd_data(&self, mut data: Buffers) -> SessionState<Buffers> {
        match std::mem::replace(&mut data.state, State::Closed) {
            State::Mail(envelope) if !envelope.rcpts.is_empty() => {
                let mailid = envelope.id.clone();
                let session = envelope.session.clone();
                let fut = self.service.send_mail(envelope).map(move |res| {
                    data.state = match res {
                        Ok(sink) => {
                            data.say_start_data_challenge();
                            State::Data(StateData {
                                session,
                                mailid,
                                sink: Box::pin(sink),
                            })
                        }
                        Err(DispatchError::Refused) => {
                            data.say_mail_queue_refused();
                            State::Connected(session)
                        }
                        Err(DispatchError::FailedTemporarily) => {
                            data.say_mail_queue_failed_temporarily();
                            State::Connected(session)
                        }
                    };
                    data
                });
                SessionState::Pending(Box::pin(fut))
            }
            state => {
                data.state = state;
                data.say_command_sequence_fail();
                SessionState::Ready(data)
            }
        }
    }
    fn cmd_rset(&self, mut data: Buffers) -> SessionState<Buffers> {
        data.say_ok();
        match data.state {
            State::New | State::Closed | State::Connected(_) => {}
            State::Mail(envelope) => {
                data.state = State::Connected(envelope.session);
            }
            State::Data(state) => {
                data.state = State::Connected(state.session);
            }
        };
        SessionState::Ready(data)
    }
    fn cmd_noop(&self, mut data: Buffers) -> SessionState<Buffers> {
        data.say_ok();
        SessionState::Ready(data)
    }
    fn cmd_starttls(&self, mut data: Buffers) -> SessionState<Buffers> {
        match data.state {
            State::Connected(ref mut state) => {
                // you cannot STARTTLS twice so we only advertise it before first use
                if state.extensions.disable(&extension::STARTTLS) {
                    let name = state.service_name.clone();
                    // TODO: better message response
                    data.say(WriteControl::StartTls(SmtpReply::ServiceReadyInfo(name)));
                } else {
                    data.say_not_implemented();
                }
            }
            _ => {
                data.say_command_sequence_fail();
            }
        }
        SessionState::Ready(data)
    }
}
