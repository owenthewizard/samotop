mod stream;

use self::stream::*;
use crate::common::*;
use crate::mail::*;
pub use samotop_model::smtp::*;

pub type InputStream = Box<dyn Stream<Item = Result<ReadControl>> + Unpin + Sync + Send>;
pub type OutputStream = Box<dyn Stream<Item = Result<WriteControl>> + Unpin + Sync + Send>;

/**
A session service handles the SMTP session.

For each connection a new handler is started with a call to `start()`.
This handler will only handle one session and then it will be dropped.

The handler will receive `ReadControl`s from the line and should produce
relevant `WriteControl`s to send down the line in response.
*/
pub trait SessionService {
    fn start(&self, input: InputStream) -> OutputStream;
}

/// Enables any clonable `MailService` to be used as a `SessionService`
///  with the default `BasicSessionHandler`
impl<S> SessionService for S
where
    S: MailService + Clone + Send + Sync + 'static,
{
    fn start(&self, input: InputStream) -> OutputStream {
        let state = SmtpState::new(self.clone());
        let handler: OutputStream = Box::new(SessionStream::new(input, state));
        handler
    }
}
