use crate::common::*;
use crate::mail::{SessionInfo, Transaction};

/**
A mail dispatch allows us to dispatch an e-mail.
For a given mail transacton it produces a Write sink that can receive mail data.
Once the sink is closed successfully, the mail is dispatched.
*/
pub trait MailDispatch: fmt::Debug {
    fn send_mail<'a, 's, 'f>(
        &'a self,
        session: &'s SessionInfo,
        transaction: Transaction,
    ) -> S1Fut<'f, DispatchResult>
    where
        'a: 'f,
        's: 'f;
}

impl<T> MailDispatch for Arc<T>
where
    T: MailDispatch,
{
    fn send_mail<'a, 's, 'f>(
        &'a self,
        session: &'s SessionInfo,
        transaction: Transaction,
    ) -> S1Fut<'f, DispatchResult>
    where
        'a: 'f,
        's: 'f,
    {
        T::send_mail(self, session, transaction)
    }
}

pub type DispatchResult = std::result::Result<Transaction, DispatchError>;

#[derive(Debug, Clone)]
pub enum DispatchError {
    Refused,
    FailedTemporarily,
}

impl std::error::Error for DispatchError {}

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match self {
            DispatchError::FailedTemporarily => write!(f, "Mail transaction failed temporarily"),
            DispatchError::Refused => write!(f, "Mail was refused by the server"),
        }
    }
}
