mod console;
pub use self::console::*;

use crate::model::mail::*;
use crate::model::Error;
use futures::prelude::*;

/**
The service which implements this trait has a name.
*/
pub trait NamedService {
    fn name(&self) -> &str;
}

/**
A mail guard can be queried whether a recepient is accepted on which address.
*/
pub trait MailGuard {
    type Future: Future<Output = AcceptRecipientResult>;
    fn accept(&self, request: AcceptRecipientRequest) -> Self::Future;
}

/**
A mail queue allows us to queue an e-mail.
We start with an envelope. Then, depending on implementation,
the `Mail` implementation receives the e-mail body.
Finally, the caller queues the mail by calling `Mail.queue()`.
*/
pub trait MailQueue {
    type Mail: Mail + Sink<bytes::Bytes, Error = Error>;
    type MailFuture: Future<Output = Option<Self::Mail>>;
    fn mail(&self, envelope: Envelope) -> Self::MailFuture;
}

/**
The final step of sending a mail is queueing it for delivery.
Calling queue should close any pending data and confirm that mail has been queued.
*/
pub trait Mail {
    fn queue_id(&self) -> &str;
}
