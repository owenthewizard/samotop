use fast_chemail::is_valid_email;
use std::ffi::OsStr;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

/// Email address
#[derive(PartialEq, Eq, Clone, Debug)]
#[cfg_attr(
    feature = "serde-impls",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub struct EmailAddress(String);

impl EmailAddress {
    pub fn new(address: String) -> EmailResult<EmailAddress> {
        if !is_valid_email(&address) && !address.ends_with("localhost") {
            return Err(Error::InvalidEmailAddress);
        }

        Ok(EmailAddress(address))
    }
}

impl From<EmailAddress> for String {
    fn from(addr: EmailAddress) -> Self {
        addr.0
    }
}

impl FromStr for EmailAddress {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        EmailAddress::new(s.to_string())
    }
}

impl Display for EmailAddress {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for EmailAddress {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
impl AsRef<[u8]> for EmailAddress {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl AsRef<OsStr> for EmailAddress {
    fn as_ref(&self) -> &OsStr {
        &self.0.as_ref()
    }
}

/// Simple email envelope representation
///
/// We only accept mailboxes, and do not support source routes (as per RFC).
#[derive(PartialEq, Eq, Clone, Debug)]
#[cfg_attr(
    feature = "serde-impls",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub struct Envelope {
    /// The envelope recipients' addresses
    ///
    /// This can not be empty.
    forward_path: Vec<EmailAddress>,
    /// The envelope sender address
    reverse_path: Option<EmailAddress>,
    /// Unique message ID to facilitate troubleshooting and matching
    message_id: String,
}

impl Envelope {
    /// Creates a new envelope, which may fail if `to` is empty.
    pub fn new(
        from: Option<EmailAddress>,
        to: Vec<EmailAddress>,
        message_id: String,
    ) -> EmailResult<Envelope> {
        if to.is_empty() {
            return Err(Error::MissingTo);
        }
        Ok(Envelope {
            forward_path: to,
            reverse_path: from,
            message_id,
        })
    }

    /// Destination addresses of the envelope
    pub fn to(&self) -> &[EmailAddress] {
        self.forward_path.as_slice()
    }

    /// Source address of the envelope
    pub fn from(&self) -> Option<&EmailAddress> {
        self.reverse_path.as_ref()
    }

    pub fn message_id(&self) -> &str {
        &self.message_id
    }
}

/// Error type for email content
#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Missing from in envelope
    #[error("missing source address")]
    MissingFrom,
    /// Missing to in envelope
    #[error("missing destination address")]
    MissingTo,
    /// Invalid email
    #[error("invalid email address")]
    InvalidEmailAddress,
}

/// Email result type
pub type EmailResult<T> = Result<T, Error>;
