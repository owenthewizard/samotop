mod handler;
mod service;
mod session;

pub use self::handler::*;
pub use self::service::*;
pub use self::session::*;
use crate::common::*;
use crate::model::io::*;
use crate::service::mail::*;
use crate::service::session::SessionService;

/// Implement this trait to override the way commands are handled
/// in stateful session service
pub trait SessionHandler {
    type Data: Default;
    fn pop(&self, data: &mut Self::Data) -> Option<WriteControl>;
    fn handle(&self, data: Self::Data, control: ReadControl) -> SessionState<Self::Data>;
}

pub type SessionFuture<T> = Pin<Box<dyn Future<Output = T> + Send + Sync>>;
pub enum SessionState<T> {
    Ready(T),
    Pending(SessionFuture<T>),
}

/// Enables any clonable `MailService` to be used as a `SessionService`
///  with the default `BasicSessionHandler`
impl<I, S> SessionService<I> for S
where
    I: Stream<Item = Result<ReadControl>>,
    S: MailService + Clone,
{
    type Session = session::StatefulSession<I, BasicSessionHandler<Self>>;
    type StartFuture = future::Ready<Self::Session>;
    fn start(&self, input: I) -> Self::StartFuture {
        let handler = BasicSessionHandler::from(self.clone());
        future::ready(StatefulSession::new(input, handler))
    }
}
