mod connection;
mod service;
pub mod tls;

pub use self::connection::*;
pub use self::service::*;

use crate::common::*;

pub trait Io: io::Read + io::Write + Sync + Send + Unpin {}
impl<T> Io for T where T: io::Read + io::Write + Sync + Send + Unpin {}
