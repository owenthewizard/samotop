mod session;

pub use self::session::*;
use crate::common::*;
use crate::mail::*;
use crate::session::*;
use crate::smtp::ReadControl;
use crate::smtp::SmtpStateBase;

/// Enables any clonable `MailService` to be used as a `SessionService`
///  with the default `BasicSessionHandler`
impl<I, S> SessionService<I> for S
where
    I: Stream<Item = Result<ReadControl>> + Unpin + Send + Sync + 'static,
    S: MailService + Clone + Send + Sync + 'static,
{
    fn start(&self, input: I) -> SessionStream {
        let state = SmtpStateBase::new(self.clone());
        let handler: SessionStream = Box::new(StatefulSession::new(input, state));
        handler
    }
}
