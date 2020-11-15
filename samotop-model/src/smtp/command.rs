use crate::{
    common::*,
    smtp::{extension, session::SmtpSession, SmtpReply},
};
use std::fmt;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

pub trait SmtpSessionCommand {
    fn apply<'s, 'f>(
        self,
        session: &'s mut dyn SmtpSession,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + 'f>>
    where
        's: 'f;
    fn verb(&self) -> &str;
}
pub trait SmtpSessionResponse {
    fn code() -> u16;
}

impl SmtpSessionCommand for SmtpHelo {
    fn apply<'s, 'f>(
        self,
        session: &'s mut dyn SmtpSession,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + 'f>>
    where
        's: 'f,
    {
        let local = session.transaction().session.service_name.clone();
        let remote = self.host().to_string();
        let extensions = session
            .transaction()
            .session
            .extensions
            .iter()
            .map(String::from)
            .collect();
        let reply = match self {
            SmtpHelo::Helo(_) => SmtpReply::OkHeloInfo {
                local,
                remote,
                extensions: vec![],
            },
            SmtpHelo::Ehlo(_) | SmtpHelo::Lhlo(_) => SmtpReply::OkHeloInfo {
                local,
                remote,
                extensions,
            },
        };
        session.transaction_mut().reset();
        session.transaction_mut().session.smtp_helo = Some(self);
        session.say(&reply)
    }

    fn verb(&self) -> &str {
        match self {
            SmtpHelo::Helo(_) => "HELO",
            SmtpHelo::Ehlo(_) => "EHLO",
            SmtpHelo::Lhlo(_) => "LHLO",
        }
    }
}

impl SmtpSessionCommand for StartTls {
    fn apply<'s, 'f>(
        self,
        session: &'s mut dyn SmtpSession,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + 'f>>
    where
        's: 'f,
    {
        let fut = async move {
            // you cannot STARTTLS twice so we only advertise it before first use
            if session
                .transaction_mut()
                .session
                .extensions
                .disable(&extension::STARTTLS)
            {
                let name = session.transaction().session.service_name.clone();
                session.transaction_mut().reset();
                // TODO: better message response
                session.say(&SmtpReply::ServiceReadyInfo(name)).await?;
                session.start_tls().await?;
            } else {
                session
                    .say(&SmtpReply::CommandNotImplementedFailure)
                    .await?;
            };
            Ok(())
        };
        Box::pin(fut)
    }

    fn verb(&self) -> &str {
        "STARTTLS"
    }
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct StartTls;

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
    Lhlo(SmtpHost),
}

impl SmtpHelo {
    pub fn is_extended(&self) -> bool {
        use self::SmtpHelo::*;
        match self {
            Helo(_) => false,
            Ehlo(_) => true,
            Lhlo(_) => true,
        }
    }
    pub fn host(&self) -> &SmtpHost {
        use self::SmtpHelo::*;
        match self {
            Helo(ref host) => host,
            Ehlo(ref host) => host,
            Lhlo(ref host) => host,
        }
    }
    pub fn name(&self) -> String {
        format!("{}", self.host())
    }
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
