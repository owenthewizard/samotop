use crate::common::*;
use crate::io::tls::MayBeTls;
use crate::io::ConnectionInfo;
use crate::io::IoService;

/// Logs an incomming connection on info level and that's it.
#[doc = "Dummy TCP service for testing samotop server"]
#[derive(Clone, Debug)]
pub struct DummyService;

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
