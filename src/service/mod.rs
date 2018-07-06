pub mod console;
pub mod dead;
pub mod samotop;

use model::session::Session;

/** Handles TCP connections */
pub trait TcpService2 {
    type Handler;
    fn start(&self) -> Self::Handler;
}

/** Handles mail sending and has a name */
pub trait MailService {
    type MailDataWrite;
    fn name(&mut self) -> &str;
    fn send(&mut self, session: &Session) -> Option<Self::MailDataWrite>;
}
