use crate::common::*;
use crate::io::ConnectionInfo;
use crate::io::IoService;

#[doc = "Dummy TCP service for testing samotop server"]
#[derive(Clone, Debug)]
pub struct DummyService;

impl<IO> IoService<IO> for DummyService {
    fn handle(&self, _io: Result<IO>, conn: ConnectionInfo) -> S3Fut<Result<()>> {
        info!("Received connection {}", conn);
        Box::pin(future::ready(Ok(())))
    }
}
