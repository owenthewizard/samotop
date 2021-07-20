use crate::io::ConnectionInfo;
use crate::smtp::*;
use std::time::Instant;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionInfo {
    /// Description of the underlying connection
    pub connection: ConnectionInfo,
    /// ESMTP extensions enabled for this session
    pub extensions: ExtensionSet,
    /// The name of the service serving this session
    pub service_name: String,
    /// The name of the peer as introduced by the HELO command
    pub peer_name: Option<String>,
    /// records the last instant a command was received
    pub last_command_at: Instant,
    /// whether the SMTP initial banner has been sent (220 service ready)
    pub banner_sent: bool,
}

impl SessionInfo {
    pub fn new(connection: ConnectionInfo, service_name: String) -> Self {
        Self {
            connection,
            service_name,
            ..Default::default()
        }
    }
}

impl Default for SessionInfo {
    fn default() -> Self {
        SessionInfo {
            last_command_at: Instant::now(),
            connection: Default::default(),
            extensions: Default::default(),
            service_name: Default::default(),
            peer_name: Default::default(),
            banner_sent: false,
        }
    }
}

impl std::fmt::Display for SessionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(
            f,
            "Client {:?} using service {} with extensions {}. {}",
            self.peer_name,
            self.service_name,
            self.extensions
                .iter()
                .fold(String::new(), |s, r| s + format!("{}, ", r).as_ref()),
            self.connection
        )
    }
}
