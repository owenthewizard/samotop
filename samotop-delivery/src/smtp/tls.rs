use async_native_tls::{TlsConnector, TlsStream};
use async_std::io::{Read, Write};
use samotop_async_trait::async_trait;

pub trait TlsProvider<T> {
    type Upgrade: TlsUpgrade<T> + Sync + Send;
    fn get(&self) -> Self::Upgrade;
}

#[derive(Debug, Copy, Clone)]
pub struct DefaultTls;

impl<T> TlsProvider<T> for DefaultTls
where
    T: Read + Write + Send + Sync + Unpin + 'static,
{
    type Upgrade = TlsConnector;
    fn get(&self) -> Self::Upgrade {
        TlsConnector::default()
    }
}

#[async_trait]
pub trait TlsUpgrade<T> {
    type Encrypted: 'static + Read + Write + Unpin + Send + Sync;
    #[future_is[Sync+'static]]
    async fn upgrade_to_tls(self, stream: T, name: String) -> std::io::Result<Self::Encrypted>;
    fn is_enabled(&self) -> bool;
}

#[async_trait]
impl<T> TlsUpgrade<T> for TlsConnector
where
    T: Read + Write + Unpin + Send + Sync + 'static,
{
    type Encrypted = TlsStream<T>;
    #[future_is[Sync+'static]]
    async fn upgrade_to_tls(self, stream: T, name: String) -> std::io::Result<Self::Encrypted> {
        self.connect(name, stream)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::BrokenPipe, e))
    }
    fn is_enabled(&self) -> bool {
        true
    }
}
