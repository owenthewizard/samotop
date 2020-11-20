mod command;
mod commands;
pub mod extension;
mod extensions;
mod reply;
mod state;

pub use self::command::*;
pub use self::extensions::*;
pub use self::reply::*;
pub use self::state::*;
use crate::mail::SessionInfo;
use std::fmt;

/// Represents the instructions for the client side of the stream.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum WriteControl {
    /// The stream should be shut down.
    Shutdown(SmtpReply),
    /// Tell codec to start data
    StartData(SmtpReply),
    /// Tell stream to upgrade to TLS
    StartTls(SmtpReply),
    /// Send an SMTP reply
    Reply(SmtpReply),
}

/// Represents the instructions for the server side of the stream.
#[derive(PartialEq, Eq, Clone)]
pub enum ReadControl {
    /** Peer connected */
    PeerConnected(SessionInfo),
    /** Peer disconnected */
    PeerShutdown,
    /** SMTP command line */
    Command(SmtpCommand, Vec<u8>),
    /** raw input that could not be understood */
    Raw(Vec<u8>),
    /** Available mail data without signalling dots */
    MailDataChunk(Vec<u8>),
    /** The SMTP data terminating dot (. CR LF) is part of protocol signalling and not part of data  */
    EndOfMailData(Vec<u8>),
    /** The SMTP data escape dot (.) is part of protocol signalling and not part of data */
    EscapeDot(Vec<u8>),
    /// Empty line or white space
    Empty(Vec<u8>),
}

impl fmt::Debug for ReadControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn write_text_or_bytes(f: &mut fmt::Formatter<'_>, inp: &[u8]) -> fmt::Result {
            if let Ok(text) = std::str::from_utf8(inp) {
                write!(f, "{:?}", text)
            } else {
                write!(f, "{:?}", inp)
            }
        }
        match self {
            ReadControl::PeerConnected(sess) => write!(f, "PeerConnected({:?})", sess),
            ReadControl::PeerShutdown => write!(f, "PeerShutdown"),
            ReadControl::Command(c, b) => {
                write!(f, "Command({:?}, ", c)?;
                write_text_or_bytes(f, b)?;
                write!(f, ")")
            }
            ReadControl::Raw(b) => {
                write!(f, "Raw(")?;
                write_text_or_bytes(f, b)?;
                write!(f, ")")
            }
            ReadControl::MailDataChunk(b) => {
                write!(f, "MailDataChunk(")?;
                write_text_or_bytes(f, b)?;
                write!(f, ")")
            }
            ReadControl::EndOfMailData(b) => {
                write!(f, "EndOfMailData(")?;
                write_text_or_bytes(f, b)?;
                write!(f, ")")
            }
            ReadControl::EscapeDot(b) => {
                write!(f, "EscapeDot(")?;
                write_text_or_bytes(f, b)?;
                write!(f, ")")
            }
            ReadControl::Empty(b) => {
                write!(f, "Empty(")?;
                write_text_or_bytes(f, b)?;
                write!(f, ")")
            }
        }
    }
}
