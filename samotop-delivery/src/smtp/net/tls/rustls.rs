use crate::{smtp::net::tls::*, SyncFuture};
use async_std::io::{Read, Write};
use async_tls::client::TlsStream;
use async_tls::TlsConnector;

#[derive(Default, Debug, Copy, Clone)]
pub struct RusTls;

impl<IO> TlsProvider<IO> for RusTls
where
    IO: Read + Write + Unpin + Send + Sync + 'static,
{
    type Upgrade = TlsConnector;
    fn get(&self) -> Option<Self::Upgrade> {
        Some(TlsConnector::default())
    }
}
impl<IO> TlsUpgrade<IO> for TlsConnector
where
    IO: Read + Write + Unpin + Send + Sync + 'static,
{
    type Encrypted = TlsStream<IO>;
    fn upgrade_to_tls(
        self,
        stream: IO,
        name: String,
    ) -> SyncFuture<'static, std::io::Result<Self::Encrypted>> {
        Box::pin(async move { self.connect(name, stream).await })
    }
}
