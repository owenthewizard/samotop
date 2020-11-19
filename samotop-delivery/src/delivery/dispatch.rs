use crate::{
    prelude::{EmailAddress, Envelope, Transport},
    MailDataStream,
};
use futures::TryFutureExt;
use samotop_model::{common::*, mail::*};
use std::error::Error as StdError;

pub struct DispatchMail<T> {
    transport: T,
}

impl<T> DispatchMail<T> {
    pub fn new(transport: T) -> Self {
        Self { transport }
    }
}

impl<T> MailDispatch for DispatchMail<T>
where
    T: Transport + Send + Sync,
    T::DataStream: Sync + Send + 'static,
    <T::DataStream as MailDataStream>::Error: StdError + Sync + Send,
{
    fn send_mail<'a, 's, 'f>(
        &'a self,
        _session: &'s SessionInfo,
        mut transaction: Transaction,
    ) -> S2Fut<'f, DispatchResult>
    where
        'a: 'f,
        's: 'f,
    {
        let fut = async move {
            let sender = transaction
                .mail
                .as_ref()
                .map(|sender| EmailAddress::new(sender.path().address()))
                .transpose()?;
            let recipients: std::result::Result<Vec<_>, _> = transaction
                .rcpts
                .iter()
                .map(|rcpt| EmailAddress::new(rcpt.address()))
                .collect();

            let envelope =
                Envelope::new(sender, recipients?, transaction.id.clone()).map_err(Error::from)?;
            trace!("Starting mail transaction.");
            match self.transport.send_stream(envelope).await {
                Ok(stream) => transaction.sink = Some(Box::pin(stream)),
                Err(e) => {
                    return Err(e.into());
                }
            }

            Ok(transaction)
        };
        let fut = fut.map_err(|e: Error| {
            error!("Failed to start mail: {:?}", e);
            DispatchError::FailedTemporarily
        });

        Box::pin(fut)
    }
}
