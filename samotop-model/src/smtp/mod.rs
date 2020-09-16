mod command;
//mod commands;
pub mod extension;
mod extensions;
mod reply;

pub use self::command::*;
pub use self::extensions::*;
pub use self::reply::*;
use crate::mail::SessionInfo;

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
#[derive(Debug, PartialEq, Eq, Clone)]
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
