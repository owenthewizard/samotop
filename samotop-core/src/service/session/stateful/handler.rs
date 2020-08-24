use crate::common::*;
use crate::model::io::*;
use crate::model::mail::*;
use crate::model::session::*;
use crate::model::smtp::*;
use crate::protocol::sink::SinkFutureExt;
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
                    sink,
                    mailid,
                    connection,
                    peer_helo,
                } = state;
                let fut = sink.send(bytes).map(move |res| match res {
                    Ok(sink) => {
                        data.state = State::Data(StateData {
                            sink,
                            mailid,
                            connection,
                            peer_helo,
                        });
                        data
                    }
                    Err(e) => {
                        warn!("Failed to write mail data for {} - {}", mailid, e);
                        data.state = State::Connected(StateHelo {
                            connection,
                            peer_helo,
                        });
                        // CheckMe: following this reset, we are not sending any response yet. handle_data_end should do that.
                        data
                    }
                });
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
                    sink,
                    mailid,
                    connection,
                    peer_helo,
                } = state;
                let fut = sink.close().map(move |res| match res {
                    Ok(()) => {
                        data.state = State::Connected(StateHelo {
                            connection,
                            peer_helo,
                        });
                        data.say_mail_queued(mailid.as_str());
                        data
                    }
                    Err(e) => {
                        warn!("Failed to finish mail data for {} - {}", mailid, e);
                        data.state = State::Connected(StateHelo {
                            connection,
                            peer_helo,
                        });
                        data.say_mail_queue_failed_temporarily();
                        data
                    }
                });
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
    pub fn handle_conn(&self, mut data: Buffers, connection: Connection) -> SessionState<Buffers> {
        data.state = State::Connected(StateHelo::from(connection));
        data.say_service_ready(self.service.name());
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
            |conn: &Connection| conn.extensions().iter().map(Clone::clone).collect();
        let respond = |data: &mut Buffers, exts| match extended {
            false => {
                data.say_helo(self.service.name(), remote);
            }
            true => {
                data.say_ehlo(&self.service.name(), exts, remote);
            }
        };
        match data.state {
            State::New | State::Closed => {
                data.say_command_sequence_fail();
            }
            State::Connected(ref mut state) => {
                let exts = get_extensions(&state.connection);
                state.peer_helo = Some(helo);
                respond(&mut data, exts);
            }
            State::Mail(ref mut state) => {
                let exts = get_extensions(&state.connection);
                state.peer_helo = Some(helo);
                respond(&mut data, exts);
            }
            State::Data(ref mut state) => {
                let exts = get_extensions(&state.connection);
                state.peer_helo = Some(helo);
                respond(&mut data, exts);
            }
        };
        data.rst();
        SessionState::Ready(data)
    }
    fn cmd_quit(&self, mut data: Buffers) -> SessionState<Buffers> {
        let name = self.service.name().to_owned();
        data.say_reply(SmtpReply::ClosingConnectionInfo(name));
        data.say(WriteControl::Shutdown);
        data.state = State::Closed;
        SessionState::Ready(data)
    }
    fn cmd_mail(&self, mut data: Buffers, mail: SmtpMail) -> SessionState<Buffers> {
        match std::mem::replace(&mut data.state, State::Closed) {
            State::Connected(state) if state.peer_helo.is_some() => {
                let mailid = self.service.new_id();

                let request = AcceptSenderRequest {
                    name: self.service.name().to_owned(),
                    local: state.connection.local_addr(),
                    peer: state.connection.peer_addr(),
                    helo: state.peer_helo.clone(),
                    mail: Some(mail.clone()),
                    id: mailid.clone(),
                };
                let fut = self.service.accept_sender(request).map(move |res| {
                    data.state = match res {
                        AcceptSenderResult::Failed => {
                            data.say_reply(SmtpReply::ProcesingError);
                            State::Connected(state)
                        }
                        AcceptSenderResult::Rejected => {
                            data.say_recipient_not_accepted();
                            State::Connected(state)
                        }
                        AcceptSenderResult::Accepted => {
                            data.say_ok();
                            State::Mail(StateMail::from((state, mail, mailid)))
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
                data.say_command_sequence_fail().rst();
                SessionState::Ready(data)
            }
        }
    }
    fn cmd_rcpt(&self, mut data: Buffers, rcpt: SmtpPath) -> SessionState<Buffers> {
        match std::mem::replace(&mut data.state, State::Closed) {
            State::Mail(mut state) => {
                let request = AcceptRecipientRequest {
                    name: self.service.name().to_owned(),
                    local: state.connection.local_addr(),
                    peer: state.connection.peer_addr(),
                    helo: state.peer_helo.clone(),
                    mail: Some(state.mail.clone()),
                    id: state.mailid.clone(),
                    rcpt: rcpt,
                };
                let fut = self.service.accept_recipient(request).map(move |res| {
                    match res {
                        AcceptRecipientResult::Failed => data.say_reply(SmtpReply::ProcesingError),
                        AcceptRecipientResult::Rejected => data.say_recipient_not_accepted(),
                        AcceptRecipientResult::RejectedWithNewPath(path) => {
                            data.say_recipient_not_local(path)
                        }
                        AcceptRecipientResult::Accepted(path) => {
                            state.recipients.push(path);
                            data.say_ok()
                        }
                        AcceptRecipientResult::AcceptedWithNewPath(path) => {
                            state.recipients.push(path.clone());
                            data.say_ok_recipient_not_local(path)
                        }
                    };
                    data.state = State::Mail(state);
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
            State::Mail(state) if !state.recipients.is_empty() => {
                let envelope = Envelope {
                    name: self.service.name().to_owned(),
                    local: state.connection.local_addr(),
                    peer: state.connection.peer_addr(),
                    helo: state.peer_helo.clone(),
                    mail: Some(state.mail.clone()),
                    id: state.mailid.to_string(),
                    rcpts: state.recipients.clone(),
                };
                let fut = self.service.mail(envelope).map(move |res| {
                    match res {
                        Some(mail) => {
                            data.state = State::Data(StateData::from((state, mail)));
                            data.say(WriteControl::StartData(SmtpReply::StartMailInputChallenge));
                        }
                        None => {
                            data.state = State::Mail(state);
                            data.say_mail_queue_refused().rst();
                        }
                    }
                    data
                });
                SessionState::Pending(Box::pin(fut))
            }
            state => {
                data.state = state;
                data.say_command_sequence_fail().rst();
                SessionState::Ready(data)
            }
        }
    }
    fn cmd_rset(&self, mut data: Buffers) -> SessionState<Buffers> {
        data.say_ok().rst();
        SessionState::Ready(data)
    }
    fn cmd_noop(&self, mut data: Buffers) -> SessionState<Buffers> {
        data.say_ok();
        SessionState::Ready(data)
    }
    fn cmd_starttls(&self, mut data: Buffers) -> SessionState<Buffers> {
        match data.state {
            State::Connected(ref mut state) => {
                let name = self.service.name().to_owned();
                // you cannot STARTTLS twice so we only advertise it before first use
                if state
                    .connection
                    .extensions_mut()
                    .disable(SmtpExtension::STARTTLS.code)
                {
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
