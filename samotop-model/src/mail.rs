use crate::io::ConnectionInfo;
use crate::smtp::*;

/// Mail envelope before sending mail data
#[derive(Default, Debug, Clone)]
pub struct Transaction {
    /// Description of the current session
    pub session: SessionInfo,
    /// unique mail transaction identifier
    pub id: String,
    /// The SMTP mail from:path sent by peer
    pub mail: Option<SmtpMail>,
    /// A list of SMTP rcpt to:path sent by peer
    pub rcpts: Vec<SmtpPath>,
    /// Extra headers prepended to the e-mail
    pub extra_headers: String,
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
    /// The SMTP helo sent by peer
    pub smtp_helo: Option<SmtpHelo>,
}

impl SessionInfo {
    pub fn new(connection: ConnectionInfo, service_name: String) -> Self {
        Self {
            connection,
            service_name,
            extensions: ExtensionSet::new(),
            smtp_helo: None,
        }
    }
}

impl std::fmt::Display for SessionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(
            f,
            "Client {:?} using service {} with extensions {}. {}",
            self.smtp_helo
                .as_ref()
                .map(|h| h.name())
                .unwrap_or_else(|| "without helo".to_owned()),
            self.service_name,
            self.extensions
                .iter()
                .fold(String::new(), |s, r| s + format!("{}, ", r).as_ref()),
            self.connection
        )
    }
}

#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub struct AddRecipientRequest {
    /// The envelope to add to
    pub transaction: Transaction,
    /// The SMTP rcpt to:path sent by peer we want to check
    pub rcpt: SmtpPath,
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum AddRecipientResult {
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

pub type DispatchResult<T> = std::result::Result<T, DispatchError>;

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
