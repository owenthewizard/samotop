//! Error and result type for file transport

use async_std::io;

/// An enum of all error kinds.
#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    /// Internal client error
    #[error("client error: {0}")]
    Client(&'static str),
    /// IO error
    #[error("io error: {0}")]
    Io(String),
}

impl From<&'static str> for Error {
    fn from(string: &'static str) -> Error {
        Error::Client(string)
    }
}
impl From<io::Error> for Error {
    fn from(error: io::Error) -> Error {
        From::from(&error)
    }
}
impl From<&io::Error> for Error {
    fn from(error: &io::Error) -> Error {
        Error::Io(format!("IO error: {:?}", error))
    }
}

/// SMTP result type
pub type StubResult = Result<(), Error>;
