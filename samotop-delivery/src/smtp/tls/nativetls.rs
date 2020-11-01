use crate::{smtp::tls::*, SyncFuture};
use async_native_tls::{TlsConnector, TlsStream};
use async_std::io::{Read, Write};

#[derive(Default, Debug, Copy, Clone)]
pub struct NativeTls;

impl<T> TlsProvider<T> for NativeTls
where
    T: Read + Write + Send + Sync + Unpin + 'static,
{
    type Upgrade = TlsConnector;
    fn get(&self) -> Option<Self::Upgrade> {
        Some(TlsConnector::default())
    }
}

impl<T> TlsUpgrade<T> for TlsConnector
where
    T: Read + Write + Unpin + Send + Sync + 'static,
{
    type Encrypted = TlsStream<T>;
    fn upgrade_to_tls(
        self,
        stream: T,
        name: String,
    ) -> SyncFuture<'static, std::io::Result<Self::Encrypted>> {
        Box::pin(async move {
            self.connect(name, stream)
                .await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::BrokenPipe, e))
        })
    }
}
