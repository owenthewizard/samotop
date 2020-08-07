use crate::model::smtp::*;
use bytes::Bytes;
use std::net::SocketAddr;

/// Represents the instructions for the client side of the stream.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum WriteControl {
    /// The stream should be shut down.
    Shutdown,
    /// Tell codec to start data
    StartData,
    /// Tell stream to upgrade to TLS
    StartTls,
    /// Send an SMTP reply
    Reply(SmtpReply),
    /// Something got done, but we should call back again
    NoOp,
}

/// Represents the instructions for the server side of the stream.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ReadControl {
    /** Peer connected */
    PeerConnected(Connection),
    /** Peer disconnected */
    PeerShutdown,
    /** SMTP command line */
    Command(SmtpCommand),
    /** raw input that could not be understood */
    Raw(Bytes),
    /** Available mail data without signalling dots */
    MailDataChunk(Bytes),
    /** The SMTP data terminating dot (. CR LF) is part of protocol signalling and not part of data  */
    EndOfMailData(Bytes),
    /** The SMTP data escape dot (.) is part of protocol signalling and not part of data */
    EscapeDot(Bytes),
    /// No control sent in given interval
    NoOp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Connection {
    pub local_addr: Option<SocketAddr>,
    pub peer_addr: Option<SocketAddr>,
}

impl std::fmt::Display for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        if let Some(a) = self.peer_addr {
            write!(f, "{}", a)?;
        } else {
            write!(f, "Unknown")?;
        }
        write!(f, " -> ")?;
        if let Some(a) = self.local_addr {
            write!(f, "{}", a)?;
        } else {
            write!(f, "Unknown")?;
        }
        Ok(())
    }
}
