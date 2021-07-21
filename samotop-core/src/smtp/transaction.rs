use crate::common::*;
use crate::mail::Recipient;
use crate::smtp::*;

/// Mail envelope before sending mail data
#[derive(Default)]
pub struct Transaction {
    /// unique mail transaction identifier
    pub id: String,
    /// The SMTP mail from:path sent by peer
    pub mail: Option<command::SmtpMail>,
    /// A list of SMTP rcpt to:path sent by peer
    pub rcpts: Vec<Recipient>,
    /// Extra headers prepended to the e-mail
    pub extra_headers: String,
    /// Write sink to write the mail into
    pub sink: Option<Pin<Box<dyn MailDataSink>>>,
    /// Special mode used to switch parsers
    pub mode: Option<&'static str>,
}

impl Transaction {
    /// Special mode where classic SMTP data are expected,
    /// used after reading some data without CRLF to keep track of the dot state
    pub const DATA_PARTIAL_MODE: &'static str = "DATA_PARTIAL";
    /// Special mode where classic SMTP data are expected
    pub const DATA_MODE: &'static str = "DATA";

    // Resets the SMTP mail transaction buffers
    // leaves SMTP helo as is
    // leaves connection info as is
    pub fn reset(&mut self) {
        self.id = String::new();
        self.mail = None;
        self.rcpts = vec![];
        self.extra_headers = String::new();
    }
    pub fn is_expecting_commands(&self) -> bool {
        self.mode.is_none() || self.sink.is_none()
    }
    pub fn is_empty(&self) -> bool {
        let Transaction {
            ref id,
            ref mail,
            ref rcpts,
            ref extra_headers,
            ref sink,
            ref mode,
        } = self;
        id.is_empty()
            && mail.is_none()
            && rcpts.is_empty()
            && extra_headers.is_empty()
            && sink.is_none()
            && mode.is_none()
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
            ref mode,
        } = self;
        f.debug_struct("Transaction")
            .field("id", id)
            .field("mail", mail)
            .field("rcpts", rcpts)
            .field("extra_headers", extra_headers)
            .field("sink", &"*")
            .field("mode", mode)
            .finish()
    }
}

pub trait MailDataSink: Write + Send + Sync + 'static {}
impl<T> MailDataSink for T where T: Write + Send + Sync + 'static {}
