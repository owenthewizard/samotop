use bytes::Bytes;
use model::command::SmtpCommand;
use std::net::SocketAddr;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ServerControll {
    /** Peer connected */
    PeerConnected {
        local: Option<SocketAddr>,
        peer: Option<SocketAddr>,
    },
    /** Peer disconnected */
    PeerShutdown,
    /** SMTP command line */
    Command(SmtpCommand),
    /** raw input that could not be understood */
    Invalid(Bytes),
    /** Available mail data without signalling dots */
    DataChunk(Bytes),
    /** The SMTP data terminating dot (. CR LF) is part of protocol signalling and not part of data  */
    FinalDot(Bytes),
    /** The SMTP data escape dot (.) is part of protocol signalling and not part of data */
    EscapeDot(Bytes),
}
