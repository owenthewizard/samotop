use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionInfo {
    pub local_addr: String,
    pub peer_addr: String,
    pub established: Instant,
}

impl ConnectionInfo {
    pub fn new(local_addr: String, peer_addr: String) -> Self {
        ConnectionInfo {
            local_addr,
            peer_addr,
            established: Instant::now(),
        }
    }
    pub fn age(&self) -> Duration {
        Instant::now() - self.established
    }
}
impl Default for ConnectionInfo {
    fn default() -> Self {
        ConnectionInfo::new(String::default(), String::default())
    }
}

impl std::fmt::Display for ConnectionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "Connection from peer ")?;
        if self.peer_addr.is_empty() {
            f.write_str("Unknown")?;
        } else {
            f.write_str(self.peer_addr.as_str())?;
        }
        write!(f, " to local ")?;
        if self.local_addr.is_empty() {
            f.write_str("Unknown")?;
        } else {
            f.write_str(self.local_addr.as_str())?;
        }
        write!(f, " established {:?} ago.", self.age())?;
        Ok(())
    }
}
