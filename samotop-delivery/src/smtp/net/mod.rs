//! Traits and impls to represent and establish network-like streams
pub mod tls;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use self::unix::*;

mod child;
mod inet;
pub use self::child::*;
pub use self::inet::*;

use self::tls::DefaultTls;
use crate::smtp::extension::ServerInfo;
use crate::{smtp::authentication::Authentication, SyncFuture};
use crate::{smtp::extension::ClientId, smtp::ClientSecurity};
use async_std::io::{self, Read, Write};
use samotop_core::io::tls::MayBeTls;
use std::fmt;
use std::time::Duration;

pub trait Connector: fmt::Debug + Sync + Send {
    type Stream: MayBeTls + Read + Write + Unpin + Sync + Send + 'static;
    /// This provider of connectivity takes care of resolving
    /// given address (which could be an IP, FQDN, URL...),
    /// establishing a connection and enabling (or not) TLS upgrade.

    fn connect<'s, 'c, 'a, C: ConnectionConfiguration>(
        &'s self,
        configuration: &'c C,
    ) -> SyncFuture<'a, io::Result<Self::Stream>>
    where
        's: 'a,
        'c: 'a;
}

pub trait ConnectionConfiguration: Sync + Send {
    fn address(&self) -> String;
    fn timeout(&self) -> Duration;
    fn security(&self) -> ClientSecurity;
    fn hello_name(&self) -> ClientId;
    fn max_reuse_count(&self) -> u16;
    fn get_authentication(
        &self,
        server_info: &ServerInfo,
        encrypted: bool,
    ) -> Option<Box<dyn Authentication>>;
    fn lmtp(&self) -> bool;
}

pub type DefaultConnector = inet::TcpConnector<DefaultTls>;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TlsMode {
    Tls,
    StartTls,
}
