mod console;
mod dirmail;
use crate::common::*;
use crate::model::io::Connection;
use crate::model::mail::*;
use crate::model::smtp::SmtpExtension;
use crate::model::Error;
pub use console::*;
pub use dirmail::*;

/**
The service which implements this trait has a name.
*/
pub trait NamedService {
    fn name(&self) -> &str;
}

/**
The service which implements this trait has a name.
*/
pub trait EsmtpService {
    fn extend(&self, connection: &mut Connection);
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
For a given mail envelope it produces a Sink that can receive mail data.
Once the sink is closed successfully, the mail is queued.
*/
pub trait MailQueue {
    type Mail: Sink<bytes::Bytes, Error = Error>;
    type MailFuture: Future<Output = Option<Self::Mail>>;
    fn mail(&self, envelope: Envelope) -> Self::MailFuture;
}
