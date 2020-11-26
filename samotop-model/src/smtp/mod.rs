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
#[derive()]
pub enum ReadControl {
    /** Peer connected */
    PeerConnected(SessionInfo),
    /** Peer disconnected */
    PeerShutdown,
    /** SMTP command line */
    Command(Box<dyn SmtpSessionCommand>, Vec<u8>),
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
        #[derive(Debug)]
        enum TB<'a> {
            T(&'a str),
            B(&'a [u8]),
        }
        fn tb(inp: &[u8]) -> TB {
            if let Ok(text) = std::str::from_utf8(inp) {
                TB::T(text)
            } else {
                TB::B(inp)
            }
        }
        match self {
            ReadControl::PeerConnected(sess) => write!(f, "PeerConnected({:?})", sess),
            ReadControl::PeerShutdown => write!(f, "PeerShutdown"),
            ReadControl::Command(c, b) => f
                .debug_tuple("Command")
                .field(&c.verb())
                .field(&tb(b))
                .finish(),
            ReadControl::Raw(b) => f.debug_tuple("Raw").field(&tb(b)).finish(),
            ReadControl::MailDataChunk(b) => f.debug_tuple("MailDataChunk").field(&tb(b)).finish(),
            ReadControl::EndOfMailData(b) => f.debug_tuple("EndOfMailData").field(&tb(b)).finish(),
            ReadControl::EscapeDot(b) => f.debug_tuple("EscapeDot").field(&tb(b)).finish(),
            ReadControl::Empty(b) => f.debug_tuple("Empty").field(&tb(b)).finish(),
        }
    }
}
