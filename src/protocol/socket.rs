use std::io::Result;
use std::net::SocketAddr;
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_core::net::TcpStream;

pub trait NetSocket: AsyncRead + AsyncWrite {
    fn peer_addr(&self) -> Result<SocketAddr>;
    fn local_addr(&self) -> Result<SocketAddr>;
}

impl NetSocket for TcpStream {
    fn peer_addr(&self) -> Result<SocketAddr> {
        self.peer_addr()
    }
    fn local_addr(&self) -> Result<SocketAddr> {
        self.local_addr()
    }
}
