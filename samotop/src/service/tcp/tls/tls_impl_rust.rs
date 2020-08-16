use crate::common::*;
use crate::service::tcp::tls::TlsProvider;
use async_tls::server::TlsStream;
use async_tls::Accept;
use async_tls::TlsAcceptor;

pub fn provide_rustls(acceptor: TlsAcceptor) -> Provider<TlsAcceptor> {
    Provider(acceptor)
}

impl<IO> TlsProvider<IO> for Provider<TlsAcceptor>
where
    IO: Read + Write + Unpin,
{
    type EncryptedIO = TlsStream<IO>;
    type UpgradeFuture = Accept<IO>;
    fn upgrade_to_tls(&self, io: IO) -> Self::UpgradeFuture {
        self.0.accept(io)
    }
}
