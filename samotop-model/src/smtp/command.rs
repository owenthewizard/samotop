use std::fmt;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

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
    /// Commandoutside ofthe base implementation.
    /// First string is the command verb, next the parameters
    Other(String, Vec<String>),
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

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum SmtpHelo {
    Helo(SmtpHost),
    Ehlo(SmtpHost),
}

impl SmtpHelo {
    pub fn is_extended<'a>(&'a self) -> bool {
        use self::SmtpHelo::*;
        match self {
            Helo(_) => false,
            Ehlo(_) => true,
        }
    }
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
        match *self {
            SmtpPath::Direct(ref addr) => match addr {
                SmtpAddress::Mailbox(ref name, ref host) => write!(f, "<{}@{}>", name, host),
            },
            SmtpPath::Null => write!(f, "<>"),
            SmtpPath::Postmaster => write!(f, "<POSTMASTER>"),
            SmtpPath::Relay(_, ref addr) => match addr {
                SmtpAddress::Mailbox(ref name, ref host) => write!(f, "<{}@{}>", name, host),
            },
        }
    }
}

impl fmt::Display for SmtpHost {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use self::SmtpHost::*;
        match *self {
            Domain(ref h) => f.write_str(h),
            Ipv4(ref h) => write!(f, "{}", h),
            Ipv6(ref h) => write!(f, "{}", h),
            Invalid {
                ref label,
                ref literal,
            } => write!(f, "{}:{}", label, literal),
            Other {
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
    Mail(SmtpPath, Vec<String>),
    Send(SmtpPath, Vec<String>),
    Saml(SmtpPath, Vec<String>),
    Soml(SmtpPath, Vec<String>),
}

impl SmtpMail {
    pub fn from(&self) -> &SmtpPath {
        match self {
            SmtpMail::Mail(p, _) => &p,
            SmtpMail::Send(p, _) => &p,
            SmtpMail::Saml(p, _) => &p,
            SmtpMail::Soml(p, _) => &p,
        }
    }
}
