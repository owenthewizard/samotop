use bytes::Bytes;
use std::io;
use tokio_proto::streaming::Body;
use super::request::*;

pub enum Act {
    Mail(Mail),
    Conn(Conn),
}

pub struct Mail {
    conn: Conn,
    from: SmtpPath,
    mail: SmtpMail,
    data: Body<Bytes, io::Error>,
}

pub struct Conn {
    helo: SmtpHelo,
    connection: SmtpConnection,
}

pub enum ActError {
    Denied,
    Failed,
}

pub type ActResult = Result<(), ActError>;
