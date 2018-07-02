use service::SamotopService;
use tokio::net::TcpStream;

#[derive(Clone)]
struct DeadService;

impl SamotopService for DeadService {
    fn handle(self, _socket: TcpStream) {}
}
