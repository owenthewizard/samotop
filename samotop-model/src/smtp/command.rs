pub use super::commands::*;
use super::SmtpReply;
use crate::{common::*, smtp::state::SmtpState};
use std::fmt;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

pub trait SmtpSessionCommand: Sync + Send + fmt::Debug {
    fn verb(&self) -> &str;
    #[must_use = "apply must be awaited"]
    fn apply(&self, state: SmtpState) -> S2Fut<SmtpState>;
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum SmtpCommand {
    StartTls,
    Helo(SmtpHelo),
    Mail(SmtpMail),
    Rcpt(SmtpRcpt),
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

    fn apply(&self, state: SmtpState) -> S2Fut<SmtpState> {
        use SmtpCommand as C;
        match self {
            C::Helo(helo) => helo.apply(state),
            C::Mail(mail) => mail.apply(state),
            C::Rcpt(rcpt) => rcpt.apply(state),
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

impl<T, E> SmtpSessionCommand for std::result::Result<T, E>
where
    T: SmtpSessionCommand,
    E: fmt::Debug + Sync + Send,
{
    fn verb(&self) -> &str {
        ""
    }

    fn apply(&self, mut state: SmtpState) -> S2Fut<SmtpState> {
        match self {
            Ok(command) => Box::pin(async move { command.apply(state).await }),
            Err(e) => {
                error!("reading SMTP input failed: {:?}", e);
                state.say_shutdown(SmtpReply::ProcesingError);
                Box::pin(ready(state))
            }
        }
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
