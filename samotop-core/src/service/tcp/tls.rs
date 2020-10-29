use crate::common::*;
use crate::model::io::ConnectionInfo;
use crate::protocol::tls::{TlsCapable, TlsDisabled};
use crate::service::tcp::TcpService;

pub trait TlsProviderFactory<IO> {
    type Provider: TlsProvider<IO> + Sync + Send;
    fn get(&self) -> Option<Self::Provider>;
}
pub trait TlsProvider<IO> {
    type EncryptedIO: Read + Write + Unpin + Sync + Send;
    type UpgradeFuture: Future<Output = std::io::Result<Self::EncryptedIO>> + Sync + Send;
    fn upgrade_to_tls(&self, io: IO) -> Self::UpgradeFuture;
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

#[async_trait]
impl<T, IO, P> TcpService<IO> for TlsEnabled<T, P>
where
    T: TcpService<TlsCapable<IO, P::Provider>> + Send + Sync,
    IO: Read + Write + Unpin + Sync + Send,
    P: TlsProviderFactory<IO> + Send + Sync,
{
    #[future_is[Send + Sync + 'static]]
    async fn handle(&self, io: Result<IO>, conn: ConnectionInfo) -> Result<()> {
        let provider = self.provider.get();

        let tls = match io {
            Ok(io) => Ok(match provider {
                Some(provider) => TlsCapable::yes(io, provider),
                None => TlsCapable::no(io),
            }),
            Err(e) => Err(e),
        };
        let fut = self.wrapped.handle(tls, conn);
        async_setup_ready!();
        fut.await
    }
}
