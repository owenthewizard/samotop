use model::command::*;
use std::net::SocketAddr;

#[derive(Debug)]
pub struct Envelope {
    pub name: String,
    pub peer: Option<SocketAddr>,
    pub local: Option<SocketAddr>,
    pub helo: Option<SmtpHelo>,
    pub mail: Option<SmtpMail>,
    pub rcpts: Vec<SmtpPath>,
}
