pub mod echo;
pub mod dead;

use tokio::net::TcpStream;

pub trait SamotopService {
    fn handle(self, TcpStream);
}
