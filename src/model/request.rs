use bytes::Bytes;
use std::net::{SocketAddr, Ipv4Addr, Ipv6Addr};

use self::SmtpInput::*;

#[derive(Eq, PartialEq, Debug)]
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

#[derive(Eq, PartialEq, Debug)]
pub enum SmtpCommand {
    Unknown(Bytes),
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
