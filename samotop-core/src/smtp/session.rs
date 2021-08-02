use crate::io::ConnectionInfo;
use crate::smtp::*;
use std::any::{Any, TypeId};
use std::collections::HashMap;

#[derive(Debug)]
pub struct SessionInfo {
    /// Description of the underlying connection
    pub connection: ConnectionInfo,
    /// ESMTP extensions enabled for this session
    pub extensions: ExtensionSet,
    /// The name of the service serving this session
    pub service_name: String,
    /// The name of the peer as introduced by the HELO command
    pub peer_name: Option<String>,
    /// Output to be processed by a driver - responses and IO controls
    pub output: Vec<DriverControl>,
    /// Input to be interpretted
    pub input: Vec<u8>,
    /// Extension-specific value store
    pub store: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
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
            connection: Default::default(),
            extensions: Default::default(),
            service_name: Default::default(),
            peer_name: Default::default(),
            input: vec![],
            output: vec![],
            store: HashMap::new(),
        }
    }
}

impl std::fmt::Display for SessionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(
            f,
            "Client {:?} using service {} with extensions {} on {}. There are {} input bytes and {} output items pending, {} items in the store.",
            self.peer_name,
            self.service_name,
            self.extensions
                .iter()
                .fold(String::new(), |s, r| s + format!("{}, ", r).as_ref()),
            self.connection,
            self.input.len(),
            self.output.len(),
            self.store.len()
        )
    }
}
