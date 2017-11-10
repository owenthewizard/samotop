use bytes::Bytes;
use std::io;
use futures::{Stream, Poll};
use tokio_proto::streaming::Body;
use super::request::*;

pub fn new_mail(
    conn: SmtpConnection,
    helo: SmtpHelo,
    mail: SmtpMail,
    rcpt: Vec<SmtpPath>,
    //data: R,
) -> Mail {
    Mail {
        conn,
        helo,
        mail,
        rcpt,
        //data: Data { inner: Body::from(Bytes::from(data)) },
    }
}

#[derive(Debug)]
pub enum Act {
    Mail(Mail),
}

#[derive(Debug)]
pub struct Mail {
    pub conn: SmtpConnection,
    pub helo: SmtpHelo,
    pub mail: SmtpMail,
    pub rcpt: Vec<SmtpPath>,
    //pub data: Data,
}

#[derive(Debug)]
pub struct Data {
    inner: Body<Bytes, io::Error>,
}

#[derive(Debug)]
pub enum ActError {
    Denied,
    Failed,
}

pub type ActResult = Result<Act, ActError>;

impl Stream for Data {
    type Item = Bytes;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Bytes>, io::Error> {
        self.inner.poll()
    }
}
