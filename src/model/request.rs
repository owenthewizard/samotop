use bytes::Bytes;
use std::net::{SocketAddr, Ipv4Addr, Ipv6Addr};

#[derive(Eq, PartialEq, Debug)]
pub enum SmtpInput {
    Command(usize, usize, SmtpCommand),
    Invalid(usize, usize, String),
    InvalidBytes(usize, usize, Bytes),
    None(usize, usize, String),

    Connect(SmtpConnection),
    Disconnect,

    StreamStart(usize),
    StreamEnd(usize),
    StreamData(usize, usize, Bytes),
}

#[derive(Eq, PartialEq, Debug)]
pub enum SmtpCommand {
    Unknown(String),
    Connect(SmtpConnection),
    Disconnect,
    Stream,
    EndOfStream,

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

#[derive(Eq, PartialEq, Debug)]
pub enum SmtpHelo {
    Helo(SmtpHost),
    Ehlo(SmtpHost),
}

#[derive(Eq, PartialEq, Debug)]
pub struct SmtpConnection {
    pub local_addr: Option<SocketAddr>,
    pub peer_addr: Option<SocketAddr>,
}

#[derive(Eq, PartialEq, Debug)]
pub enum SmtpMail {
    Mail(SmtpPath),
    Send(SmtpPath),
    Saml(SmtpPath),
    Soml(SmtpPath),
}
