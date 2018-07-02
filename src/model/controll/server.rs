use bytes::Bytes;
use std::net::SocketAddr;

#[derive(Debug)]
pub enum ServerControll {
    PeerConnected(SocketAddr),
    PeerShutdown(SocketAddr),
    Command(String),
    Invalid(Bytes),
    Data(Bytes),
    DataEnd(Bytes),
    DataDot(Bytes)
}
