use crate::smtp::net::tls::TlsCapable;
use crate::smtp::net::tls::{TlsProvider, TlsUpgrade};
use crate::smtp::net::Connector;
use crate::smtp::net::TlsMode;
use crate::{smtp::net::ConnectionConfiguration, SyncFuture};
use async_std::io;
use async_std::os::unix::net::UnixStream;
use samotop_model::common::Pin;
use samotop_model::io::tls::MayBeTls;

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
            let to = configuration.address();
            let timeout = configuration.timeout();

            let stream = io::timeout(timeout, UnixStream::connect(&to)).await?;
            let stream = Box::new(stream);
            let mut stream = match self.provider.get() {
                Some(u) => TlsCapable::enabled(stream, Box::new(u), String::default()),
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
