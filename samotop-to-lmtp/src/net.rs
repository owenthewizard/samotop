use async_tls::client::TlsStream;
use async_tls::TlsConnector;
use samotop_async_trait::async_trait;
use samotop_core::common::*;
use samotop_delivery::smtp::net::*;
use samotop_delivery::smtp::tls::*;

pub type MyCon = TcpConnector<MyTls>;
pub struct MyTls;
pub struct MyUpgrade(TlsConnector);

pub fn conn() -> MyCon {
    TcpConnector {
        tls_mode: TlsMode::StartTls,
        provider: MyTls,
    }
}

impl<IO> TlsProvider<IO> for MyTls
where
    IO: Read + Write + Unpin + Send + Sync + 'static,
{
    type Upgrade = MyUpgrade;
    fn get(&self) -> Self::Upgrade {
        MyUpgrade(TlsConnector::default())
    }
}
#[async_trait]
impl<IO> TlsUpgrade<IO> for MyUpgrade
where
    IO: Read + Write + Unpin + Send + Sync + 'static,
{
    type Encrypted = TlsStream<IO>;
    #[future_is[Sync+'static]]
    async fn upgrade_to_tls(self, stream: IO, name: String) -> std::io::Result<Self::Encrypted> {
        self.0.connect(name, stream).await
    }
    fn is_enabled(&self) -> bool {
        true
    }
}
