use service::*;
use tokio::io;
use tokio::net::TcpStream;
use tokio::prelude::*;

#[derive(Clone, Debug)]
pub struct DeadService;

impl TcpService for DeadService {
    fn handle(self, _socket: TcpStream) {}
}
impl TcpService2 for DeadService {
    type Handler = Self;
    fn start(&self) -> Self::Handler {
        self.clone()
    }
}

impl Sink for DeadService {
    type SinkItem = TcpStream;
    type SinkError = io::Error;

    fn start_send(&mut self, _item: Self::SinkItem) -> io::Result<AsyncSink<Self::SinkItem>> {
        info!("got an item");
        Ok(AsyncSink::Ready)
    }

    fn poll_complete(&mut self) -> Result<Async<()>, io::Error> {
        Ok(Async::Ready(()))
    }
}
