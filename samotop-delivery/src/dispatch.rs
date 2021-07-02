use crate::prelude::{EmailAddress, Envelope, Transport};
use samotop_core::{common::*, mail::*};
use std::fmt;

#[derive(Debug)]
pub struct DispatchMail<T> {
    transport: T,
}

impl<T> DispatchMail<T> {
    pub fn new(transport: T) -> Self
    where
        T: fmt::Debug,
    {
        Self { transport }
    }
}

impl<T> MailDispatch for DispatchMail<T>
where
    T: Transport + Send + Sync,
    T::DataStream: Sync + Send + 'static,
    T::Error: std::error::Error + Sync + Send + 'static,
{
    fn send_mail<'a, 's, 'f>(
        &'a self,
        _session: &'s SessionInfo,
        transaction: Transaction,
    ) -> S1Fut<'f, DispatchResult>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
            match start_mail(&self.transport, transaction).await {
                Err(e) => {
                    error!("Failed to start mail: {:?}", e);
                    Err(DispatchError::FailedTemporarily)
                }
                Ok(r) => Ok(r),
            }
        })
    }
}

async fn start_mail<T>(transport: &T, mut transaction: Transaction) -> Result<Transaction>
where
    T: Transport + Send + Sync,
    T::DataStream: Sync + Send + 'static,
    T::Error: std::error::Error + Sync + Send + 'static,
{
    let sender = transaction
        .mail
        .as_ref()
        .map(|mail| EmailAddress::new(mail.sender().address()))
        .transpose()?;
    let recipients: std::result::Result<Vec<_>, _> = transaction
        .rcpts
        .iter()
        .map(|rcpt| EmailAddress::new(rcpt.address.address()))
        .collect();

    let envelope =
        Envelope::new(sender, recipients?, transaction.id.clone()).map_err(Error::from)?;
    trace!("Starting downstream mail transaction.");
    let stream = transport.send_stream(envelope).await?;
    transaction.sink = Some(Box::pin(stream));

    Ok(transaction)
}
