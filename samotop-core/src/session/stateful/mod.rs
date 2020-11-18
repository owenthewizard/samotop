mod handler;
mod service;
mod session;

pub use self::handler::*;
pub use self::service::*;
pub use self::session::*;
use crate::common::*;
use crate::mail::*;
use crate::session::*;
use crate::smtp::{ReadControl, WriteControl};

/// Implement this trait to override the way commands are handled
/// in stateful session service
pub trait SessionHandler {
    type Data: Default;
    fn pop(&self, data: &mut Self::Data) -> Option<WriteControl>;
    fn handle(&self, data: Self::Data, control: ReadControl) -> S3Fut<Self::Data>;
}

/// Enables any clonable `MailService` to be used as a `SessionService`
///  with the default `BasicSessionHandler`
impl<I, S> SessionService<I> for S
where
    I: Stream<Item = Result<ReadControl>> + Unpin + Send + Sync + 'static,
    S: MailService + Clone + Send + Sync + 'static,
{
    fn start(&self, input: I) -> SessionStream {
        let handler = BasicSessionHandler::from(self.clone());
        let handler: SessionStream = Box::new(StatefulSession::new(input, handler));
        handler
    }
}
