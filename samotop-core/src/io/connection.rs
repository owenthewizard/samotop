use crate::common::time_based_id;
use std::time::{Duration, SystemTime};

/// Carries connection infromation (TCP, unix socket, ...) so that remaining code can abstract away from it as Io
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectionInfo {
    pub id: String,
    pub local_addr: String,
    pub peer_addr: String,
    pub established: SystemTime,
}

impl ConnectionInfo {
    pub fn new(local_addr: String, peer_addr: String) -> Self {
        ConnectionInfo {
            id: time_based_id(),
            local_addr,
            peer_addr,
            established: SystemTime::now(),
        }
    }
    pub fn age(&self) -> Duration {
        self.established.elapsed().unwrap_or(Duration::ZERO)
    }
}
impl Default for ConnectionInfo {
    fn default() -> Self {
        ConnectionInfo::new(String::default(), String::default())
    }
}

impl std::fmt::Display for ConnectionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "connection id {} from peer ", self.id)?;
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
        write!(f, " established {}s ago", self.age().as_secs_f64())?;
        Ok(())
    }
}

#[cfg(test)]
mod store_tests {
    use regex::Regex;

    use super::*;

    #[test]
    pub fn display_connection_info() {
        let mut sut = ConnectionInfo::default();
        sut.established = SystemTime::UNIX_EPOCH;
        let dump = sut.to_string();
        let dump = Regex::new("[0-9]+")
            .expect("regex")
            .replace_all(&dump, "--redaced--");
        insta::assert_display_snapshot!(dump, @"connection id --redaced-- from peer Unknown to local Unknown established --redaced--.--redaced--s ago");
    }

    #[test]
    pub fn timely_connection_info() {
        let sut = ConnectionInfo::default();
        assert!(sut.established.elapsed().expect("duration").as_secs() < 1)
    }

    #[test]
    pub fn unique_connection_info() {
        let sut1 = ConnectionInfo::default();
        let sut2 = ConnectionInfo::default();
        assert_ne!(sut1.id, sut2.id);
    }
}
