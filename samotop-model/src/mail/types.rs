use crate::common::{Pin, Write};
use crate::io::ConnectionInfo;
use crate::smtp::*;
use std::{
    fmt,
    time::{Duration, Instant},
};

/// Mail envelope before sending mail data
#[derive(Default)]
pub struct Transaction {
    /// unique mail transaction identifier
    pub id: String,
    /// The SMTP mail from:path sent by peer
    pub mail: Option<SmtpMail>,
    /// A list of SMTP rcpt to:path sent by peer
    pub rcpts: Vec<SmtpPath>,
    /// Extra headers prepended to the e-mail
    pub extra_headers: String,
    /// Write sink to write the mail into
    pub sink: Option<Pin<Box<dyn MailDataSink>>>,
}
pub type StartMailRequest = Transaction;

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct SessionInfo {
    /// Description of the underlying connection
    pub connection: ConnectionInfo,
    /// ESMTP extensions enabled for this session
    pub extensions: ExtensionSet,
    /// The name of the service serving this session
    pub service_name: String,
    /// The SMTP helo sent by peer - only the command verb, such as HELO, EHLO, LHLO
    pub smtp_helo: Option<String>,
    /// The name of the peer as introduced by the HELO command
    pub peer_name: Option<String>,
    /// records the last instant a command was received
    pub last_command_at: Option<Instant>,
    /// How long in total do we wait for a command?
    pub command_timeout: Duration,
}

impl Transaction {
    // Resets the SMTP mail transaction buffers
    // leaves SMTP helo as is
    // leaves connection info as is
    pub fn reset(&mut self) {
        self.id = String::new();
        self.mail = None;
        self.rcpts = vec![];
        self.extra_headers = String::new();
    }
    pub fn is_empty(&self) -> bool {
        let Transaction {
            ref id,
            ref mail,
            ref rcpts,
            ref extra_headers,
            ref sink,
        } = self;
        id.is_empty()
            && mail.is_none()
            && rcpts.is_empty()
            && extra_headers.is_empty()
            && sink.is_none()
    }
}

impl fmt::Debug for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Transaction {
            ref id,
            ref mail,
            ref rcpts,
            ref extra_headers,
            sink: _sink,
        } = self;
        f.debug_struct("Transaction")
            .field("id", id)
            .field("mail", mail)
            .field("rcpts", rcpts)
            .field("extra_headers", extra_headers)
            .field("sink", &"*")
            .finish()
    }
}

impl SessionInfo {
    pub fn new(connection: ConnectionInfo, service_name: String) -> Self {
        Self {
            connection,
            service_name,
            ..Default::default()
        }
    }
}

impl std::fmt::Display for SessionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(
            f,
            "Client {:?} using service ({}) {} with extensions {}. {}",
            self.peer_name,
            self.smtp_helo
                .as_ref()
                .map(String::as_str)
                .unwrap_or_else(|| "without helo"),
            self.service_name,
            self.extensions
                .iter()
                .fold(String::new(), |s, r| s + format!("{}, ", r).as_ref()),
            self.connection
        )
    }
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum StartMailResult {
    /// Failure with explanation that should include the ID
    Failed(StartMailFailure, String),
    /// 250 Mail command accepted
    Accepted(Transaction),
}

#[derive(Debug, Clone)]
pub enum StartMailFailure {
    /// The whole mail transaction failed, subsequent RCPT and DATA will fail
    /// 421  <domain> Service not available, closing transmission channel
    ///  (This may be a reply to any command if the service knows it must
    ///    shut down)
    TerminateSession,
    /// 550 Requested action not taken: mailbox unavailable (e.g., mailbox
    /// not found, no access, or command rejected for policy reasons)
    Rejected,
    /// 553  Requested action not taken: mailbox name not allowed (e.g.,
    /// mailbox syntax incorrect)
    InvalidSender,
    /// 552  Requested mail action aborted: exceeded storage allocation
    StorageExhaustedPermanently,
    /// 452  Requested action not taken: insufficient system storage
    StorageExhaustedTemporarily,
    /// 451  Requested action aborted: local error in processing
    FailedTemporarily,
    /// 555  MAIL FROM/RCPT TO parameters not recognized or not implemented
    InvalidParameter,
    /// 455  Server unable to accommodate parameters
    InvalidParameterValue,
}

/// Request to check if mail is accepted for given recipient
#[derive(Debug)]
pub struct AddRecipientRequest {
    /// The envelope to add to
    pub transaction: Transaction,
    /// The SMTP rcpt to:path sent by peer we want to check
    pub rcpt: SmtpPath,
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum AddRecipientResult {
    Inconclusive(AddRecipientRequest),
    /// The whole mail transaction failed, subsequent RCPT and DATA will fail
    /// 421  <domain> Service not available, closing transmission channel
    ///  (This may be a reply to any command if the service knows it must
    ///    shut down)
    TerminateSession(String),
    /// Failed with description that should include the ID, see `AddRecipientFailure`
    Failed(Transaction, AddRecipientFailure, String),
    /// 251  User not local; will forward to <forward-path>
    AcceptedWithNewPath(Transaction, SmtpPath),
    /// 250  Requested mail action okay, completed
    Accepted(Transaction),
}

#[derive(Debug, Clone)]
pub enum AddRecipientFailure {
    /// 550 Requested action not taken: mailbox unavailable (e.g., mailbox
    /// not found, no access, or command rejected for policy reasons)
    RejectedPermanently,
    /// 450  Requested mail action not taken: mailbox unavailable (e.g.,
    /// mailbox busy or temporarily blocked for policy reasons)
    RejectedTemporarily,
    /// 551  User not local; please try <forward-path> (See Section 3.4)
    Moved(SmtpPath),
    /// 553  Requested action not taken: mailbox name not allowed (e.g.,
    /// mailbox syntax incorrect)
    InvalidRecipient,
    /// 552  Requested mail action aborted: exceeded storage allocation
    StorageExhaustedPermanently,
    /// 452  Requested action not taken: insufficient system storage
    StorageExhaustedTemporarily,
    /// 451  Requested action aborted: local error in processing
    FailedTemporarily,
    /// 555  MAIL FROM/RCPT TO parameters not recognized or not implemented
    InvalidParameter,
    /// 455  Server unable to accommodate parameters
    InvalidParameterValue,
}

pub trait MailDataSink: Write + Send + Sync + 'static {}
impl<T> MailDataSink for T where T: Write + Send + Sync + 'static {}

pub type DispatchResult = std::result::Result<Transaction, DispatchError>;

#[derive(Debug, Clone)]
pub enum DispatchError {
    Refused,
    FailedTemporarily,
}

impl std::error::Error for DispatchError {}

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            DispatchError::FailedTemporarily => write!(f, "Mail transaction failed temporarily"),
            DispatchError::Refused => write!(f, "Mail was refused by the server"),
        }
    }
}
