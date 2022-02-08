use crate::{
    common::*,
    smtp::SmtpSession,
    config::{Component, ComposableComponent, MultiComponent},
};

/**
A mail dispatch allows us to dispatch an e-mail.
For a given mail transacton it puts a Write sink for receiving mail data into the Transaction.
Once the sink is closed successfully, the mail is dispatched.
*/
pub trait MailDispatch: fmt::Debug {
    /// Add a mail data sink to mail transaction.
    ///
    /// This call may fail for various reasons, resulting in
    /// permanent or temporary refusal to send the mail after
    /// the DATA command.
    ///
    /// Finishing the mail transaction is marked by sucessfully closing the mail data sink.
    fn open_mail_body<'a, 's, 'f>(
        &'a self,
        session: &'s mut SmtpSession,
    ) -> S2Fut<'f, DispatchResult>
    where
        'a: 'f,
        's: 'f;
}

pub struct MailDispatchService {}
impl Component for MailDispatchService {
    type Target = Arc<dyn MailDispatch + Send + Sync>;
}
impl MultiComponent for MailDispatchService {}
impl ComposableComponent for MailDispatchService {
    fn from_none() -> Self::Target {
        Self::from_many(vec![])
    }

    fn from_many(options: Vec<Self::Target>) -> Self::Target {
        Arc::new(options)
    }
}

impl MailDispatch for Vec<<MailDispatchService as Component>::Target> {
    fn open_mail_body<'a, 's, 'f>(
        &'a self,
        session: &'s mut SmtpSession,
    ) -> S2Fut<'f, DispatchResult>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
            for dispatch in self {
                trace!("open_mail_body calling {:?}", dispatch);
                dispatch.open_mail_body(session).await?;
            }
            FallBack.open_mail_body(session).await
        })
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

impl MailDispatch for FallBack {
    /// Succeeds if the sink is already set, otherwise fails
    fn open_mail_body<'a, 's, 'f>(
        &'a self,
        session: &'s mut SmtpSession,
    ) -> S2Fut<'f, DispatchResult>
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
