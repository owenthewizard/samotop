pub use super::commands::*;
use super::ReadControl;
use crate::{common::*, parser::ParseError, smtp::state::SmtpState};
use std::fmt;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

pub trait SmtpSessionCommand: Sync + Send + fmt::Debug {
    fn verb(&self) -> &str;
    #[must_use = "apply must be awaited"]
    fn apply<'a>(&'a self, state: SmtpState) -> S2Fut<'a, SmtpState>;
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum SmtpCommand {
    StartTls,
    Helo(SmtpHelo),
    Mail(SmtpMail),
    Rcpt(SmtpPath),
    Expn(String),
    Vrfy(String),
    Help(Vec<String>),
    Noop(Vec<String>),
    Quit,
    Rset,
    Data,
    Turn,
    /// Command outside of the base implementation.
    /// First string is the command verb, next the parameters
    Other(String, Vec<String>),
}

impl SmtpSessionCommand for SmtpCommand {
    fn verb(&self) -> &str {
        use SmtpCommand as C;
        match self {
            C::Helo(helo) => helo.verb(),
            C::Mail(mail) => mail.verb(),
            C::Rcpt(_) => "RCPT",
            C::Data => SmtpData.verb(),
            C::Quit => SmtpQuit.verb(),
            C::Rset => SmtpRset.verb(),
            C::Noop(_) => SmtpNoop.verb(),
            C::StartTls => StartTls.verb(),
            C::Expn(_) => "EXPN",
            C::Vrfy(_) => "VRFY",
            C::Help(_) => "HELP",
            C::Turn => "TURN",
            C::Other(verb, _) => verb.as_str(),
        }
    }

    fn apply<'a>(&'a self, state: SmtpState) -> S2Fut<'a, SmtpState> {
        use SmtpCommand as C;
        match self {
            C::Helo(helo) => helo.apply(state),
            C::Mail(mail) => mail.apply(state),
            C::Rcpt(path) => {
                Box::pin(async move { SmtpRcpt::from(path.clone()).apply(state).await })
            }
            C::Data => SmtpData.apply(state),
            C::Quit => SmtpQuit.apply(state),
            C::Rset => SmtpRset.apply(state),
            C::Noop(_) => SmtpNoop.apply(state),
            C::StartTls => StartTls.apply(state),
            C::Expn(_) | C::Vrfy(_) | C::Help(_) | C::Turn | C::Other(_, _) => {
                Box::pin(async move { SmtpUnknownCommand::default().apply(state).await })
            }
        }
    }
}

impl SmtpSessionCommand for ReadControl {
    fn verb(&self) -> &str {
        match self {
            ReadControl::PeerConnected(sess) => sess.verb(),
            ReadControl::PeerShutdown => SessionShutdown.verb(),
            ReadControl::Raw(_) => "",
            ReadControl::Command(cmd, _) => cmd.verb(),
            ReadControl::MailDataChunk(_) => "",
            ReadControl::EndOfMailData(_) => MailBodyEnd.verb(),
            ReadControl::Empty(_) => "",
            ReadControl::EscapeDot(_) => "",
        }
    }

    fn apply<'a>(&'a self, mut state: SmtpState) -> S2Fut<'a, SmtpState> {
        Box::pin(async move {
            if !state.reads.is_empty() {
                // previous raw control left some bytes behind
                match self {
                    ReadControl::Raw(_) => {
                        // ok, parsing will carry on
                    }
                    _ => {
                        // nope, we will not parse the leftover, let's say so.
                        state.reads.clear();
                        state = SmtpInvalidCommand::default().apply(state).await;
                    }
                }
            }

            match self {
                ReadControl::PeerConnected(sess) => sess.apply(state).await,
                ReadControl::PeerShutdown => SessionShutdown.apply(state).await,
                ReadControl::Command(cmd, _) => cmd.apply(state).await,
                ReadControl::MailDataChunk(bytes) => MailBodyChunk(bytes).apply(state).await,
                ReadControl::EndOfMailData(_) => MailBodyEnd.apply(state).await,
                ReadControl::Empty(_) => state,
                ReadControl::EscapeDot(_) => state,
                ReadControl::Raw(b) => {
                    state.reads.extend_from_slice(b.as_slice());

                    loop {
                        break if state.reads.is_empty() {
                            state
                        } else {
                            match state.service.parse_command(state.reads.as_slice()) {
                                Ok((remaining, command)) => {
                                    trace!(
                                        "Parsed {} bytes - a {} command",
                                        state.reads.len() - remaining.len(),
                                        command.verb()
                                    );
                                    state.reads = remaining.to_vec();
                                    state = command.apply(state).await;
                                    continue;
                                }
                                Err(ParseError::Incomplete) => {
                                    // we will need more bytes...
                                    state
                                }
                                Err(e) => {
                                    warn!(
                                        "Parser did not match, passing current line as is {} long. {:?}",
                                        state.reads.len(), e
                                    );
                                    state.reads.clear();
                                    SmtpInvalidCommand::default().apply(state).await
                                }
                            }
                        };
                    }
                }
            }
        })
    }
}
#[derive(Eq, PartialEq, Debug, Clone)]
pub enum SmtpHost {
    Domain(String),
    Ipv4(Ipv4Addr),
    Ipv6(Ipv6Addr),
    Invalid { label: String, literal: String },
    Other { label: String, literal: String },
}

impl SmtpHost {
    pub fn domain(&self) -> String {
        match self {
            SmtpHost::Domain(s) => s.clone(),
            SmtpHost::Ipv4(ip) => format!("{}", ip),
            SmtpHost::Ipv6(ip) => format!("{}", ip),
            SmtpHost::Invalid { label, literal } => format!("{}:{}", label, literal),
            SmtpHost::Other { label, literal } => format!("{}:{}", label, literal),
        }
    }
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum SmtpPath {
    Direct(SmtpAddress),
    Relay(Vec<SmtpHost>, SmtpAddress),
    Postmaster,
    Null,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum SmtpAddress {
    Mailbox(String, SmtpHost),
}

impl SmtpPath {
    pub fn address(&self) -> String {
        match *self {
            SmtpPath::Direct(ref addr) => match addr {
                SmtpAddress::Mailbox(ref name, ref host) => format!("{}@{}", name, host),
            },
            SmtpPath::Null => String::new(),
            SmtpPath::Postmaster => "POSTMASTER".to_owned(),
            SmtpPath::Relay(_, ref addr) => match addr {
                SmtpAddress::Mailbox(ref name, ref host) => format!("{}@{}", name, host),
            },
        }
    }
}

impl fmt::Display for SmtpPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<{}>", self.address())
    }
}

impl fmt::Display for SmtpHost {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::SmtpHost::*;
        match *self {
            Domain(ref h) => f.write_str(h),
            Ipv4(ref h) => write!(f, "[{}]", h),
            Ipv6(ref h) => write!(f, "[IPv6:{}]", h),
            Invalid {
                ref label,
                ref literal,
            } => write!(f, "[{}:{}]", label, literal),
            Other {
                ref label,
                ref literal,
            } => write!(f, "[{}:{}]", label, literal),
        }
    }
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct SmtpConnection {
    pub local_name: String,
    pub local_addr: Option<SocketAddr>,
    pub peer_addr: Option<SocketAddr>,
}
