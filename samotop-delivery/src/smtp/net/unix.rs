use crate::smtp::net::ConnectionConfiguration;
use crate::smtp::net::Connector;
use crate::smtp::net::MaybeTls;
use crate::smtp::net::NetworkStream;
use crate::smtp::net::State;
use crate::smtp::net::TlsMode;
use crate::smtp::tls::{DefaultTls, TlsProvider, TlsUpgrade};
use async_std::io;
use async_std::os::unix::net::UnixStream;
use samotop_async_trait::async_trait;

#[derive(Debug)]
pub struct SocksConnector<TLS> {
    pub tls_mode: TlsMode,
    pub provider: TLS,
}

impl Default for SocksConnector<DefaultTls> {
    fn default() -> Self {
        Self {
            tls_mode: TlsMode::StartTls,
            provider: DefaultTls,
        }
    }
}

#[async_trait]
impl<TLS> Connector for SocksConnector<TLS>
where
    TLS: TlsProvider<UnixStream> + Sync + Send + 'static,
{
    type Stream = NetworkStream<
        UnixStream,
        <TLS::Upgrade as TlsUpgrade<UnixStream>>::Encrypted,
        TLS::Upgrade,
    >;
    /// This provider of connectivity takes care of resolving
    /// given address (which could be an IP, FQDN, URL...),
    /// establishing a connection and enabling (or not) TLS upgrade.
    #[future_is[Sync + Send]]
    async fn connect<C: ConnectionConfiguration + Sync>(
        &self,
        configuration: &C,
    ) -> io::Result<Self::Stream> {
        let to = configuration.address();
        let timeout = configuration.timeout();

        let stream = io::timeout(timeout, UnixStream::connect(&to)).await?;

        let mut stream = NetworkStream {
            peer_addr: None,
            peer_name: to,
            state: State::Plain(stream, self.provider.get()),
        };

        match self.tls_mode {
            TlsMode::Tls => stream.encrypt()?,
            TlsMode::StartTls => { /* ready! */ }
        }
        Ok(stream)
    }
}
