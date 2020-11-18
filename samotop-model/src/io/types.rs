use std::net::SocketAddr;
use std::time::{Duration, Instant};

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
