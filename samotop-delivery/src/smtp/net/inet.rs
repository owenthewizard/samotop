use crate::smtp::net::ConnectionConfiguration;
use crate::smtp::net::Connector;
use crate::smtp::net::MaybeTls;
use crate::smtp::net::NetworkStream;
use crate::smtp::net::State;
use crate::smtp::net::TlsMode;
use crate::smtp::tls::{DefaultTls, TlsProvider, TlsUpgrade};
use async_std::io;
use async_std::net::{TcpStream, ToSocketAddrs};
use samotop_async_trait::async_trait;

#[derive(Debug)]
pub struct TcpConnector<TLS> {
    pub tls_mode: TlsMode,
    pub provider: TLS,
}

impl Default for TcpConnector<DefaultTls> {
    fn default() -> Self {
        Self {
            tls_mode: TlsMode::StartTls,
            provider: DefaultTls,
        }
    }
}

#[async_trait]
impl<TLS> Connector for TcpConnector<TLS>
where
    TLS: TlsProvider<TcpStream> + Sync + Send + 'static,
{
    type Stream =
        NetworkStream<TcpStream, <TLS::Upgrade as TlsUpgrade<TcpStream>>::Encrypted, TLS::Upgrade>;
    /// This provider of connectivity takes care of resolving
    /// given address (which could be an IP, FQDN, URL...),
    /// establishing a connection and enabling (or not) TLS upgrade.
    #[future_is[Sync + Send]]
    async fn connect<C: ConnectionConfiguration + Sync>(
        &self,
        configuration: &C,
    ) -> io::Result<Self::Stream> {
        // TODO: try alternative addresses on failure. Here we just pick the first one.
        let mut to = configuration.address();
        let timeout = configuration.timeout();
        let addr = to.to_socket_addrs().await?.next().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("No address resolved for {}", to),
            )
        })?;

        let tcp_stream = io::timeout(timeout, TcpStream::connect(addr)).await?;

        // remove port part, domain/host remains
        to.find(':').map(|i| to.split_off(i));
        let mut stream = NetworkStream {
            peer_addr: tcp_stream.peer_addr().ok(),
            peer_name: to,
            state: State::Plain(tcp_stream, self.provider.get()),
        };

        match self.tls_mode {
            TlsMode::Tls => stream.encrypt()?,
            TlsMode::StartTls => { /* ready! */ }
        }
        Ok(stream)
    }
}
