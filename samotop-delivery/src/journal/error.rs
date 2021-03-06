//! Error and result type for file transport

use async_std::io;
use lozizol::model::SequenceIdError;

/// An enum of all error kinds.
#[derive(thiserror::Error, Debug)]
pub enum JournalError {
    // /// Internal client error
    // #[error("client error: {0}")]
    // Client(&'static str),
    /// IO error
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    // /// JSON serialization error
    // #[error("serialization error: {0}")]
    // JsonSerialization(#[from] serde_json::Error),
    /// Sequence ID invalid
    #[error("sequence ID validation error: {0}")]
    SequenceId(#[from] SequenceIdError<String>),
}

/// SMTP result type
pub type JournalResult<T> = Result<T, JournalError>;
