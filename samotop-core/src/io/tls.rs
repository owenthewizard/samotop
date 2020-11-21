use crate::common::*;
use crate::io::ConnectionInfo;
use crate::io::IoService;
use crate::protocol::tls::{TlsCapable, TlsDisabled};

pub trait TlsProviderFactory<IO> {
    type Provider: TlsProvider<IO> + Sync + Send;
    fn get(&self) -> Option<Self::Provider>;
}
pub trait TlsProvider<IO> {
    type EncryptedIO: 'static + Read + Write + Unpin + Sync + Send;
    fn upgrade_to_tls(&self, io: IO) -> S3Fut<std::io::Result<Self::EncryptedIO>>;
}
impl<IO, T> TlsProviderFactory<IO> for Option<T>
where
    T: TlsProvider<IO> + Clone + Sync + Send,
{
    type Provider = T;
    fn get(&self) -> Option<Self::Provider> {
        self.as_ref().cloned()
    }
}
impl<IO, T> TlsProviderFactory<IO> for dyn AsRef<T>
where
    T: TlsProvider<IO> + Clone + Sync + Send,
{
    type Provider = T;
    fn get(&self) -> Option<Self::Provider> {
        Some(self.as_ref().clone())
    }
}

#[doc = "Dummy TCP service for testing samotop server"]
#[derive(Clone)]
pub struct TlsEnabled<T, P> {
    provider: P,
    wrapped: T,
}
impl<T> TlsEnabled<T, TlsDisabled> {
    /// Tls will not be enabled at all
    pub fn disabled(wrapped: T) -> Self {
        TlsEnabled::new(wrapped, TlsDisabled)
    }
}
impl<T, P> TlsEnabled<T, P> {
    pub fn new(wrapped: T, provider: P) -> Self {
        TlsEnabled { wrapped, provider }
    }
}

impl<T, IO, P> IoService<IO> for TlsEnabled<T, P>
where
    T: IoService<TlsCapable<IO, P::Provider>> + Send + Sync,
    IO: Read + Write + Unpin + Sync + Send,
    P: TlsProviderFactory<IO> + Send + Sync,
{
    fn handle(&self, io: Result<IO>, conn: ConnectionInfo) -> S3Fut<Result<()>> {
        let provider = self.provider.get();

        let tls = match io {
            Ok(io) => Ok(match provider {
                Some(provider) => TlsCapable::yes(io, provider),
                None => TlsCapable::no(io),
            }),
            Err(e) => Err(e),
        };
        self.wrapped.handle(tls, conn)
    }
}
