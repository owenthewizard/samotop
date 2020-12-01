use crate::smtp::net::tls::TlsCapable;
use crate::smtp::net::tls::{TlsProvider, TlsUpgrade};
use crate::smtp::net::Connector;
use crate::smtp::net::TlsMode;
use crate::{smtp::net::ConnectionConfiguration, SyncFuture};
use async_std::io;
use async_std::net::{TcpStream, ToSocketAddrs};
use samotop_model::common::Pin;
use samotop_model::io::tls::MayBeTls;

#[derive(Debug)]
pub struct TcpConnector<TLS> {
    pub tls_mode: TlsMode,
    pub provider: TLS,
}

impl<TLS: Default> Default for TcpConnector<TLS> {
    fn default() -> Self {
        Self {
            tls_mode: TlsMode::StartTls,
            provider: TLS::default(),
        }
    }
}

impl<TLS> Connector for TcpConnector<TLS>
where
    TLS: TlsProvider + Sync + Send + 'static,
{
    type Stream = TlsCapable;
    /// This provider of connectivity takes care of resolving
    /// given address (which could be an IP, FQDN, URL...),
    /// establishing a connection and enabling (or not) TLS upgrade.
    fn connect<'s, 'c, 'a, C: ConnectionConfiguration + Sync>(
        &'s self,
        configuration: &'c C,
    ) -> SyncFuture<'a, io::Result<Self::Stream>>
    where
        's: 'a,
        'c: 'a,
    {
        Box::pin(async move {
            // TODO: try alternative addresses on failure. Here we just pick the first one.
            let mut to = configuration.address();
            let timeout = configuration.timeout();
            let addr = to.to_socket_addrs().await?.next().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("No address resolved for {}", to),
                )
            })?;

            let stream = io::timeout(timeout, TcpStream::connect(addr)).await?;

            // remove port part, domain/host remains
            to.find(':').map(|i| to.split_off(i));
            let stream = Box::new(stream);
            let mut stream = match self.provider.get() {
                Some(u) => TlsCapable::enabled(stream, u, to),
                None => TlsCapable::plaintext(stream),
            };

            match self.tls_mode {
                TlsMode::Tls => Pin::new(&mut stream).encrypt(),
                TlsMode::StartTls => { /* ready! */ }
            }
            Ok(stream)
        })
    }
}
