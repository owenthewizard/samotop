use crate::service::*;
use tokio::net::TcpStream;
use tokio::prelude::future::FutureResult;
use tokio::prelude::*;

#[doc = "Dummy TCP service for samotop server"]
#[derive(Clone, Debug)]
pub struct DeadService;

impl TcpService for DeadService {
    type Future = FutureResult<(), ()>;
    fn handle(self, _stream: TcpStream) -> Self::Future {
        future::ok(())
    }
}
