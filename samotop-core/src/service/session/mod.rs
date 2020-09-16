pub mod dummy;
pub mod stateful;
use crate::common::*;
use crate::model::smtp::WriteControl;

/**
A session service handles the SMTP session.

For each connection a new handler is started with a call to `start()`.
This handler will only handle one session and then it will be dropped.

The handler will receive `ReadControl`s from the line and should produce
relevant `WriteControl`s to send down the line in response.
*/
pub trait SessionService<TIn> {
    type Session: Stream<Item = Result<WriteControl>>;
    type StartFuture: Future<Output = Self::Session>;
    fn start(&self, input: TIn) -> Self::StartFuture;
}
