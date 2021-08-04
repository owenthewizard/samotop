use crate::{common::*, smtp::SmtpSession};
use std::ops::Deref;

/**
A mail dispatch allows us to dispatch an e-mail.
For a given mail transacton it produces a Write sink that can receive mail data.
Once the sink is closed successfully, the mail is dispatched.
*/
pub trait MailDispatch: fmt::Debug {
    fn open_mail_body<'a, 's, 'f>(
        &'a self,
        session: &'s mut SmtpSession,
    ) -> S1Fut<'f, DispatchResult>
    where
        'a: 'f,
        's: 'f;
}

impl<S: MailDispatch + ?Sized, T: Deref<Target = S>> MailDispatch for T
where
    T: fmt::Debug + Send + Sync,
    S: Sync,
{
    fn open_mail_body<'a, 's, 'f>(
        &'a self,
        session: &'s mut SmtpSession,
    ) -> S1Fut<'f, DispatchResult>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move { S::open_mail_body(Deref::deref(self), session).await })
    }
}

pub type DispatchResult = std::result::Result<(), DispatchError>;

#[derive(Debug, Clone)]
pub enum DispatchError {
    Permanent,
    Temporary,
}

impl std::error::Error for DispatchError {}

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            DispatchError::Temporary => write!(f, "Mail transaction failed temporarily"),
            DispatchError::Permanent => write!(f, "Mail was refused by the server"),
        }
    }
}

impl MailDispatch for Dummy {
    /// Succeeds if the sink is already set, otherwise fails
    fn open_mail_body<'a, 's, 'f>(
        &'a self,
        session: &'s mut SmtpSession,
    ) -> S1Fut<'f, DispatchResult>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(ready(if session.transaction.sink.is_none() {
            DispatchResult::Err(DispatchError::Permanent)
        } else {
            Ok(())
        }))
    }
}
