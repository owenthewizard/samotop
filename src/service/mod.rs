pub mod console;
pub mod dead;
pub mod echo;
pub mod samotop;

use model::session::Session;
use tokio::net::TcpStream;

/** Handles TCP connections */
pub trait TcpService {
    fn handle(self, TcpStream);
}

/** Handles mail sending and has a name */
pub trait MailService {
    type MailDataWrite;
    fn name(&mut self) -> &str;
    fn send(&mut self, session: &Session) -> Option<Self::MailDataWrite>;
}
