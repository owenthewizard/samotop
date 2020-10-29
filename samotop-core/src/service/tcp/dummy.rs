use crate::common::*;
use crate::model::io::ConnectionInfo;
use crate::model::Result;
use crate::service::tcp::TcpService;

#[doc = "Dummy TCP service for testing samotop server"]
#[derive(Clone, Debug)]
pub struct DummyTcpService;

#[async_trait]
impl<IO> TcpService<IO> for DummyTcpService {
    #[future_is[Send + Sync + 'static]]
    async fn handle(&self, _io: Result<IO>, conn: ConnectionInfo) -> Result<()> {
        info!("Received connection {}", conn);
        Ok(())
    }
}
