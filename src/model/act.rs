use bytes::Bytes;
use std::io;
use futures::{Stream, Poll};
use tokio_proto::streaming::Body;
use super::request::*;

#[derive(Debug)]
pub enum Act {
    Mail(Mail),
}

#[derive(Debug)]
pub struct Mail {
    pub helo: SmtpHelo,
    pub conn: SmtpConnection,
    pub from: SmtpPath,
    pub mail: SmtpMail,
    pub data: Data,
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
