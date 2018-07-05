use service::TcpService;
use tokio::net::TcpStream;

#[derive(Clone)]
struct DeadService;

impl TcpService for DeadService {
    fn handle(self, _socket: TcpStream) {}
}
