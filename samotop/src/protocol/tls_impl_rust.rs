use crate::common::*;
use crate::service::tcp::TlsProvider;
use async_tls::server::TlsStream;
use async_tls::Accept;
use async_tls::TlsAcceptor;

impl<IO> TlsProvider<IO> for TlsAcceptor
where
    IO: Read + Write + Unpin,
{
    type EncryptedIO = TlsStream<IO>;
    type UpgradeFuture = Accept<IO>;
    fn upgrade_to_tls(&self, io: IO) -> Self::UpgradeFuture {
        self.accept(io)
    }
}
