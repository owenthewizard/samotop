mod session;

pub use self::session::*;
use crate::mail::*;
use crate::session::*;
use crate::smtp::SmtpStateBase;

/// Enables any clonable `MailService` to be used as a `SessionService`
///  with the default `BasicSessionHandler`
impl<S> SessionService for S
where
    S: MailService + Clone + Send + Sync + 'static,
{
    fn start(&self, input: InputStream) -> OutputStream {
        let state = SmtpStateBase::new(self.clone());
        let handler: OutputStream = Box::new(StatefulSession::new(input, state));
        handler
    }
}
