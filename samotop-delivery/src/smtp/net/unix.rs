use crate::smtp::net::tls::{TlsProvider, TlsUpgrade};
use crate::smtp::net::Connector;
use crate::smtp::net::MaybeTls;
use crate::smtp::net::NetworkStream;
use crate::smtp::net::State;
use crate::smtp::net::TlsMode;
use crate::{smtp::net::ConnectionConfiguration, SyncFuture};
use async_std::io;
use async_std::os::unix::net::UnixStream;

#[derive(Debug)]
pub struct UnixConnector<TLS> {
    pub tls_mode: TlsMode,
    pub provider: TLS,
}

impl<TLS: Default> Default for UnixConnector<TLS> {
    fn default() -> Self {
        Self {
            tls_mode: TlsMode::StartTls,
            provider: TLS::default(),
        }
    }
}

impl<TLS> Connector for UnixConnector<TLS>
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
    fn connect<'s, 'c, 'a, C: ConnectionConfiguration + Sync>(
        &'s self,
        configuration: &'c C,
    ) -> SyncFuture<'a, io::Result<Self::Stream>>
    where
        's: 'a,
        'c: 'a,
    {
        Box::pin(async move {
            let to = configuration.address();
            let timeout = configuration.timeout();

            let stream = io::timeout(timeout, UnixStream::connect(&to)).await?;

            let mut stream = NetworkStream {
                peer_addr: None,
                peer_name: to,
                state: match self.provider.get() {
                    Some(u) => State::Enabled(stream, u),
                    None => State::Disabled(stream),
                },
            };

            match self.tls_mode {
                TlsMode::Tls => stream.encrypt()?,
                TlsMode::StartTls => { /* ready! */ }
            }
            Ok(stream)
        })
    }
}
