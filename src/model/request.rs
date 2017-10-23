use std::net::{SocketAddr, Ipv4Addr, Ipv6Addr};

#[derive(Eq, PartialEq, Debug)]
pub enum SmtpInput {
    Command(usize, usize, SmtpCommand),
    Data(usize, usize, Vec<u8>),
    Invalid(usize, usize, String),
    None(usize, usize, String),
}

#[derive(Eq, PartialEq, Debug)]
pub enum SmtpCommand {
    Unknown(String),
    Connect {
        local_addr: Option<SocketAddr>,
        peer_addr: Option<SocketAddr>,
    },
    Disconnect,

    Ehlo(SmtpHost),
    Helo(SmtpHost),
    Mail(SmtpDelivery, SmtpPath),
    Rcpt(SmtpPath),
    Expn(String),
    Vrfy(String),
    Help(Vec<String>),
    Noop(Vec<String>),
    Quit,
    Rset,
    Data,
    Turn,
}

#[derive(Eq, PartialEq, Debug)]
pub enum SmtpDelivery {
    Mail,
    Send,
    Saml,
    Soml,
}

#[derive(Eq, PartialEq, Debug)]
pub enum SmtpHost {
    Domain(String),
    Ipv4(Ipv4Addr),
    Ipv6(Ipv6Addr),
    Invalid { label: String, literal: String },
    Other { label: String, literal: String },
}

#[derive(Eq, PartialEq, Debug)]
pub enum SmtpPath {
    Direct(SmtpAddress),
    Relay(Vec<SmtpHost>, SmtpAddress),
    Postmaster,
    Null,
}

#[derive(Eq, PartialEq, Debug)]
pub enum SmtpAddress {
    Mailbox(String, SmtpHost),
}
