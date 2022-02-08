mod connection;
mod handler;
#[cfg(feature = "server")]
mod server;
mod session;
pub mod tls;

pub use self::connection::*;
pub use self::handler::*;
#[cfg(feature = "server")]
pub use self::server::*;
pub use self::session::*;

use crate::common::*;
use crate::config::Component;
use crate::config::MultiComponent;

pub trait Io: io::Read + io::Write + Sync + Send + Unpin {}
impl<T> Io for T where T: io::Read + io::Write + Sync + Send + Unpin {}

pub trait Server {
    fn sessions<'s, 'f>(
        &'s self,
    ) -> S1Fut<'f, Result<Pin<Box<dyn Stream<Item = Result<Session>> + Send + Sync>>>>
    where
        's: 'f;
}

pub struct ServerService {}
impl Component for ServerService {
    type Target = Arc<dyn Server + Send + Sync>;
}
impl MultiComponent for ServerService {}
