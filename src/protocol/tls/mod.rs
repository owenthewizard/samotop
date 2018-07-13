#[cfg(not(feature = "tls"))]
mod tls_off;
#[cfg(feature = "tls")]
mod tls_on;

#[cfg(not(feature = "tls"))]
pub use self::tls_off::*;
#[cfg(feature = "tls")]
pub use self::tls_on::*;

use model::controll::*;
use tokio::prelude::*;

impl<S> WillDoTls for S
where
    S: Read + Write,
{
}

pub trait WillDoTls
where
    Self: Sized,
{
    fn tls(self, config: TlsWorker) -> TlsCapable<Self> {
        tls_capable(self, config)
    }
}
