use model::command::*;
use std::net::SocketAddr;

/// Mail envelope before sending mail data
#[derive(Debug)]
pub struct Envelope {
    /// Service name
    pub name: String,
    /// Local server endpoint
    pub local: Option<SocketAddr>,
    /// Remote peer endpoint
    pub peer: Option<SocketAddr>,
    /// The SMTP helo sent by peer
    pub helo: Option<SmtpHelo>,
    /// The SMTP mail from:path sent by peer
    pub mail: Option<SmtpMail>,
    /// A list of SMTP rcpt to:path sent by peer
    pub rcpts: Vec<SmtpPath>,
}

/// Request to check if mail is accepted for given recipient
#[derive(Debug)]
pub struct AcceptRecipient {
    /// Service name
    pub name: String,
    /// Local server endpoint
    pub local: Option<SocketAddr>,
    /// Remote peer endpoint
    pub peer: Option<SocketAddr>,
    /// The SMTP helo sent by peer
    pub helo: Option<SmtpHelo>,
    /// The SMTP mail from:path sent by peer
    pub mail: Option<SmtpMail>,
    /// The SMTP rcpt to:path sent by peer we want to check
    pub rcpt: SmtpPath,
}