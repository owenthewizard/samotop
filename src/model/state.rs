use std::fmt;
use std::io::{Read, Write, Error};
use bytes::Bytes;
use futures::sync::mpsc::Sender;
use tokio_proto::streaming::Body;
use model::request::*;

#[derive(Debug)]
pub struct Session;

#[derive(Debug)]
pub struct Conn {
    pub conn: SmtpConnection,
}

#[derive(Debug)]
pub struct Helo {
    pub conn: SmtpConnection,
    pub helo: SmtpHelo,
}

#[derive(Debug)]
pub struct Mail {
    pub conn: SmtpConnection,
    pub helo: SmtpHelo,
    pub mail: SmtpMail,
}

#[derive(Debug)]
pub struct Rcpt {
    pub conn: SmtpConnection,
    pub helo: SmtpHelo,
    pub mail: SmtpMail,
    pub rcpt: Vec<SmtpPath>,
}

pub struct Data {
    pub conn: SmtpConnection,
    pub helo: SmtpHelo,
    pub mail: SmtpMail,
    pub rcpt: Vec<SmtpPath>,
    pub body: Body<Bytes, Error>,
    pub tx: Sender<Result<Bytes, Error>>,
}

impl fmt::Debug for Data {
    fn fmt<'a>(&self, fmt: &mut fmt::Formatter<'a>) -> Result<(), fmt::Error> {
        fmt.write_fmt(format_args!(
            "Data conn: {:?}, helo: {:?}, mail: {:?}, rcpt: {:?}",
            self.conn,
            self.helo,
            self.mail,
            self.rcpt
        ))
    }
}

#[derive(Debug)]
pub struct Closed;
