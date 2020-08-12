use crate::model::smtp::*;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use std::net::SocketAddr;

/// Represents the instructions for the client side of the stream.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum WriteControl {
    /// The stream should be shut down.
    Shutdown,
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
    /// Empty line or white space
    Empty(Bytes),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Connection {
    local_addr: Option<SocketAddr>,
    peer_addr: Option<SocketAddr>,
    established: DateTime<Utc>,
    extensions: ExtensionSet,
}

impl Connection {
    pub fn new<L, P>(local: L, peer: P) -> Connection
    where
        L: Into<Option<SocketAddr>>,
        P: Into<Option<SocketAddr>>,
    {
        Connection {
            local_addr: local.into(),
            peer_addr: peer.into(),
            established: Utc::now(),
            extensions: ExtensionSet::new(),
        }
    }
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.local_addr.clone()
    }
    pub fn peer_addr(&self) -> Option<SocketAddr> {
        self.peer_addr.clone()
    }
    pub fn established(&self) -> DateTime<Utc> {
        self.established.clone()
    }
    pub fn extensions(&self) -> &ExtensionSet {
        &self.extensions
    }
    pub fn extensions_mut(&mut self) -> &mut ExtensionSet {
        &mut self.extensions
    }
}
impl Default for Connection {
    fn default() -> Connection {
        Connection::new(None, None)
    }
}

impl std::fmt::Display for Connection {
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
        write!(f, " established {}", self.established)?;
        Ok(())
    }
}
