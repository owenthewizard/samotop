use self::SmtpInput::*;
use bytes::Bytes;
use std::fmt;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum SmtpInput {
    Command(usize, usize, SmtpCommand),
    Invalid(usize, usize, Bytes),
    Incomplete(usize, usize, Bytes),
    None(usize, usize, String),

    Connect(SmtpConnection),
    Disconnect,

    StreamStart(usize),
    StreamEnd(usize),
    StreamData(usize, usize, Bytes),
}

impl SmtpInput {
    pub fn len(&self) -> usize {
        match self {
            &Command(_, l, _) => l,
            &Invalid(_, l, _) => l,
            &Incomplete(_, l, _) => l,
            &None(_, l, _) => l,

            &Connect(_) => 0,
            &Disconnect => 0,

            &StreamStart(_) => 0,
            &StreamEnd(_) => 0,
            &StreamData(_, l, _) => l,
        }
    }
    pub fn pos(self, pos: usize) -> Self {
        match self {
            Command(_, l, c) => SmtpInput::Command(pos, l, c),
            Invalid(_, l, d) => SmtpInput::Invalid(pos, l, d),
            Incomplete(_, l, d) => SmtpInput::Incomplete(pos, l, d),
            None(_, l, s) => SmtpInput::None(pos, l, s),

            Connect(c) => SmtpInput::Connect(c),
            Disconnect => SmtpInput::Disconnect,

            StreamStart(_) => SmtpInput::StreamStart(pos),
            StreamEnd(_) => SmtpInput::StreamEnd(pos),
            StreamData(_, l, d) => SmtpInput::StreamData(pos, l, d),
        }
    }
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum SmtpCommand {
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
    Unknown(String),
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum SmtpHost {
    Domain(String),
    Ipv4(Ipv4Addr),
    Ipv6(Ipv6Addr),
    Invalid { label: String, literal: String },
    Other { label: String, literal: String },
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

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum SmtpHelo {
    Helo(SmtpHost),
    Ehlo(SmtpHost),
}

impl SmtpHelo {
    pub fn host<'a>(&'a self) -> &'a SmtpHost {
        use self::SmtpHelo::*;
        match self {
            &Helo(ref host) => host,
            &Ehlo(ref host) => host,
        }
    }
    pub fn name(&self) -> String {
        format!("{}", self.host())
    }
}

impl fmt::Display for SmtpPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            &SmtpPath::Direct(ref addr) => match addr {
                &SmtpAddress::Mailbox(ref name, ref host) => write!(f, "<{}@{}>", name, host),
            },
            &SmtpPath::Null => write!(f, "<>"),
            &SmtpPath::Postmaster => write!(f, "<POSTMASTER>"),
            &SmtpPath::Relay(_, ref addr) => match addr {
                &SmtpAddress::Mailbox(ref name, ref host) => write!(f, "<{}@{}>", name, host),
            },
        }
    }
}

impl fmt::Display for SmtpHost {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use self::SmtpHost::*;
        match self {
            &Domain(ref h) => f.write_str(h),
            &Ipv4(ref h) => write!(f, "{}", h),
            &Ipv6(ref h) => write!(f, "{}", h),
            &Invalid {
                ref label,
                ref literal,
            } => write!(f, "{}:{}", label, literal),
            &Other {
                ref label,
                ref literal,
            } => write!(f, "{}:{}", label, literal),
        }
    }
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct SmtpConnection {
    pub local_name: String,
    pub local_addr: Option<SocketAddr>,
    pub peer_addr: Option<SocketAddr>,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum SmtpMail {
    Mail(SmtpPath),
    Send(SmtpPath),
    Saml(SmtpPath),
    Soml(SmtpPath),
}

impl SmtpMail {
    pub fn from(&self) -> &SmtpPath {
        match self {
            SmtpMail::Mail(p) => &p,
            SmtpMail::Send(p) => &p,
            SmtpMail::Saml(p) => &p,
            SmtpMail::Soml(p) => &p,
        }
    }
}
