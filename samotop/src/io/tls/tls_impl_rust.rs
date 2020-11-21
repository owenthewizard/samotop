use crate::common::*;
use crate::io::tls::TlsProvider;
use async_tls::server::TlsStream;
use async_tls::TlsAcceptor;

pub fn provide_rustls(acceptor: TlsAcceptor) -> Provider<TlsAcceptor> {
    Provider(acceptor)
}

impl<IO> TlsProvider<IO> for Provider<TlsAcceptor>
where
    IO: 'static + Read + Write + Unpin + Sync + Send,
{
    type EncryptedIO = TlsStream<IO>;
    fn upgrade_to_tls(&self, io: IO) -> S3Fut<std::io::Result<Self::EncryptedIO>> {
        Box::pin(self.0.accept(io))
    }
}
