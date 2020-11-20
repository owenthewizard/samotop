#[cfg(feature = "native-tls")]
mod nativetls;
mod notls;
#[cfg(feature = "rustls")]
mod rustls;

#[cfg(feature = "native-tls")]
pub use nativetls::*;
pub use notls::*;
#[cfg(feature = "rustls")]
pub use rustls::*;

use crate::SyncFuture;
use async_std::io::{Read, Write};

#[cfg(feature = "rustls")]
pub type DefaultTls = RusTls;
#[cfg(all(not(feature = "rustls"), feature = "native-tls"))]
pub type DefaultTls = NativeTls;
#[cfg(all(not(feature = "rustls"), not(feature = "native-tls")))]
pub type DefaultTls = NoTls;

pub trait TlsProvider<T>: std::fmt::Debug {
    type Upgrade: TlsUpgrade<T> + Sync + Send;
    fn get(&self) -> Option<Self::Upgrade>;
}

pub trait TlsUpgrade<T> {
    type Encrypted: 'static + Read + Write + Unpin + Send + Sync;
    fn upgrade_to_tls(
        self,
        stream: T,
        name: String,
    ) -> SyncFuture<'static, std::io::Result<Self::Encrypted>>;
}
