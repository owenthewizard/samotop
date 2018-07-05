pub mod dead;
pub mod echo;
pub mod mail;
pub mod console;

use model::session::Session;
use tokio::net::TcpStream;

pub trait SamotopService {
    fn handle(self, TcpStream);
}

pub trait MailService {
    type MailDataWrite;
    fn name(&mut self) -> &str;
    fn send(&mut self, session: &Session) -> Option<Self::MailDataWrite>;
}
