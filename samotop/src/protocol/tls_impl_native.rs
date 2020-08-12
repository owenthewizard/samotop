use crate::common::*;
use crate::service::tcp::TlsProvider;
use async_native_tls::TlsAcceptor;
use async_native_tls::TlsStream;

impl<IO> TlsProvider<IO> for TlsAcceptor
where
    IO: Read + Write + Unpin + Send + 'static,
{
    type EncryptedIO = TlsStream<IO>;
    type UpgradeFuture =
        Pin<Box<dyn Future<Output = std::io::Result<Self::EncryptedIO>> >>;
    fn upgrade_to_tls(&self, io: IO) -> Self::UpgradeFuture {
        // FIXME: broken, can't figure out the lifetimes
        Box::pin(self.accept(io).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("Failed to get TLS - {}", e),
            )
        }))
    }
}
