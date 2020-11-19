use crate::{variant, Config};
use samotop_core::{common::*, mail::*};
use samotop_delivery::{
    dir::FileTransport,
    prelude::{EmailAddress, Envelope, Transport},
};
use std::path::PathBuf;

pub struct Dir {
    pub path: PathBuf,
}

impl Config<variant::Dir> {
    pub fn dirmail_dispatch(path: PathBuf) -> Result<Self> {
        let variant = variant::Dir { path };
        Ok(Self { variant })
    }
}

impl MailSetup for Config<variant::Dir> {
    fn setup(self, builder: &mut Builder) {
        let transport = Arc::new(FileTransport::new(self.variant.path));
        builder
            .dispatch
            .insert(0, Box::new(DirMail::new(transport)))
    }
}

pub struct DirMail<T> {
    inner: T,
}

impl<T> DirMail<T> {
    fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl MailDispatch for DirMail<Arc<FileTransport>> {
    fn send_mail<'a, 's, 'f>(
        &'a self,
        _session: &'s SessionInfo,
        mut transaction: Transaction,
    ) -> S2Fut<'f, DispatchResult>
    where
        'a: 'f,
        's: 'f,
    {
        let transport = self.inner.clone();
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
            let stream = send_stream(transport, envelope).await?;
            transaction.sink = Some(Box::pin(stream));
            Ok(transaction)
        };
        let fut = fut.map_err(|e: Error| {
            error!("Failed to start mail: {:?}", e);
            DispatchError::FailedTemporarily
        });

        Box::pin(fut)
    }
}

/// resolves ownership/lifetime trouble by capturing the Arc
async fn send_stream(
    transport: Arc<FileTransport>,
    envelope: Envelope,
) -> Result<<FileTransport as Transport>::DataStream> {
    Ok(transport.send_stream(envelope).await?)
}
