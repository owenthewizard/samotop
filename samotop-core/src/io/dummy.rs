use crate::common::*;
use crate::io::tls::MayBeTls;
use crate::io::ConnectionInfo;
use crate::io::IoService;

pub use crate::common::Dummy as DummyService;

impl IoService for DummyService {
    fn handle(
        &self,
        _io: Result<Box<dyn MayBeTls>>,
        connection: ConnectionInfo,
    ) -> S1Fut<'static, Result<()>> {
        info!("Received connection {}", connection);
        Box::pin(ready(Ok(())))
    }
}
