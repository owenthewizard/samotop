use crate::common::*;
use crate::model::io::Connection;
use crate::model::smtp::SmtpExtension;
use crate::protocol::{TlsCapable, TlsDisabled};
use crate::service::tcp::TcpService;

pub trait TlsProvider<IO> {
    type EncryptedIO: Read + Write + Unpin;
    type UpgradeFuture: Future<Output = std::io::Result<Self::EncryptedIO>>;
    fn upgrade_to_tls(&self, io: IO) -> Self::UpgradeFuture;
}

#[doc = "Dummy TCP service for testing samotop server"]
#[derive(Clone)]
pub struct TlsEnabled<T, P> {
    provider: Option<P>,
    wrapped: T,
}
impl<T> TlsEnabled<T, TlsDisabled> {
    pub fn disabled(wrapped: T) -> Self {
        TlsEnabled::no(wrapped)
    }
}
impl<T, P> TlsEnabled<T, P> {
    pub fn yes(wrapped: T, provider: P) -> Self {
        TlsEnabled::new(wrapped, Some(provider))
    }
    pub fn no(wrapped: T) -> Self {
        TlsEnabled::new(wrapped, None)
    }
    pub fn new(wrapped: T, provider: Option<P>) -> Self {
        TlsEnabled { wrapped, provider }
    }
}

impl<T, IO, P> TcpService<IO> for TlsEnabled<T, P>
where
    T: TcpService<TlsCapable<IO, P>>,
    IO: Read + Write + Unpin,
    P: TlsProvider<IO>,
{
    type Future = T::Future;
    fn handle(self, io: Result<IO>, mut conn: Connection) -> Self::Future {
        let TlsEnabled { provider, wrapped } = self;
        let tls = if let Some(provider) = provider {
            conn.extensions_mut().enable(SmtpExtension::STARTTLS);
            io.map(|io| TlsCapable::yes(io, provider))
        } else {
            io.map(|io| TlsCapable::no(io))
        };
        wrapped.handle(tls, conn)
    }
}
