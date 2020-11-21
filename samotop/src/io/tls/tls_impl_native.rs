use crate::common::*;
use async_native_tls::TlsAcceptor;
use async_native_tls::TlsStream;
use samotop_core::io::tls::TlsProvider;

pub fn provide_native_tls(acceptor: TlsAcceptor) -> Provider<Arc<TlsAcceptor>> {
    Provider(Arc::new(acceptor))
}

impl<IO> TlsProvider<IO> for Provider<Arc<TlsAcceptor>>
where
    IO: 'static + Read + Write + Unpin + Sync + Send,
{
    type EncryptedIO = TlsStream<IO>;
    fn upgrade_to_tls(&self, io: IO) -> S3Fut<std::io::Result<Self::EncryptedIO>> {
        let acceptor = self.0.clone();
        let fut = async move {
            match acceptor.accept(io).await {
                Ok(encrypted) => Ok(encrypted),
                Err(e) => Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    format!("Failed to get TLS - {}", e),
                )),
            }
        };
        Box::pin(fut)
    }
}
