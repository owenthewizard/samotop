pub mod dummy;
mod stateful;

pub use self::stateful::*;

use crate::common::*;
use crate::model::io::WriteControl;
use crate::model::io::ReadControl;

/**
A session service handles the SMTP session.

For each connection a new handler is started with a call to `start()`.
This handler will only handle one session and then it will be dropped.

The handler will receive `ReadControl`s from the line and should produce
relevant `WriteControl`s to send down the line in response.
*/
pub trait SessionService {
    type Handler: Sink<ReadControl, Error = Error> + Stream<Item = Result<WriteControl>>;
    fn start(&self) -> Self::Handler;
}
