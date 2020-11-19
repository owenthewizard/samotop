use crate::{variant, Config};
use samotop_core::{common::*, mail::*};
use samotop_delivery::{
    prelude::{EmailAddress, Envelope, SmtpClient, SmtpTransport, Transport},
    smtp::net::Connector,
    smtp::ConnectionReuseParameters,
};

pub struct LmtpDispatch<C: Connector> {
    pub client: SmtpClient,
    pub connector: C,
}

impl<C: Connector> Config<variant::LmtpDispatch<C>> {
    pub fn lmtp_dispatch(address: String, connector: C) -> Result<Self> {
        let variant = variant::LmtpDispatch {
            client: SmtpClient::new(&address)?.lmtp(true),
            connector,
        };
        Ok(Self { variant })
    }
    pub fn reuse(mut self, lifetimes: u16) -> Self {
        self.variant.client = match lifetimes {
            0 => self
                .variant
                .client
                .connection_reuse(ConnectionReuseParameters::ReuseUnlimited),
            1 => self
                .variant
                .client
                .connection_reuse(ConnectionReuseParameters::NoReuse),
            n => self
                .variant
                .client
                .connection_reuse(ConnectionReuseParameters::ReuseLimited(n - 1)),
        };
        self
    }
}

impl<C: Connector> MailSetup for Config<variant::LmtpDispatch<C>>
where
    C: 'static,
{
    fn setup(self, builder: &mut Builder) {
        let transport = Arc::new(self.variant.client.connect_with(self.variant.connector));
        builder
            .dispatch
            .insert(0, Box::new(LmtpMail::new(transport)))
    }
}

pub struct LmtpMail<T> {
    inner: T,
}

impl<T> LmtpMail<T> {
    fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<C: Connector> MailDispatch for LmtpMail<Arc<SmtpTransport<SmtpClient, C>>>
where
    C: 'static,
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
async fn send_stream<C: Connector>(
    transport: Arc<SmtpTransport<SmtpClient, C>>,
    envelope: Envelope,
) -> Result<<SmtpTransport<SmtpClient, C> as Transport>::DataStream> {
    Ok(transport.send_stream(envelope).await?)
}
