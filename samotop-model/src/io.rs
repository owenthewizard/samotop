use crate::mail::SessionInfo;
use crate::smtp::*;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionInfo {
    pub local_addr: Option<SocketAddr>,
    pub peer_addr: Option<SocketAddr>,
    pub established: Instant,
}

impl ConnectionInfo {
    pub fn new<L, P>(local: L, peer: P) -> Self
    where
        L: Into<Option<SocketAddr>>,
        P: Into<Option<SocketAddr>>,
    {
        ConnectionInfo {
            local_addr: local.into(),
            peer_addr: peer.into(),
            established: Instant::now(),
        }
    }
    pub fn age(&self) -> Duration {
        Instant::now() - self.established
    }
}
impl Default for ConnectionInfo {
    fn default() -> Self {
        ConnectionInfo::new(None, None)
    }
}

impl std::fmt::Display for ConnectionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "Connection from peer ")?;
        if let Some(a) = self.peer_addr {
            write!(f, "{}", a)?;
        } else {
            write!(f, "Unknown")?;
        }
        write!(f, " to local ")?;
        if let Some(a) = self.local_addr {
            write!(f, "{}", a)?;
        } else {
            write!(f, "Unknown")?;
        }
        write!(f, " established {:?} ago.", self.age())?;
        Ok(())
    }
}
