#[macro_use]
extern crate log;

pub use samotop_delivery::smtp::net;

use samotop_core::{common::*, mail::*};
use samotop_delivery::{
    prelude::{EmailAddress, Envelope, MailDataStream, SmtpClient, SmtpTransport, Transport},
    smtp::net::Connector,
    smtp::ConnectionReuseParameters,
};

pub struct Config<Variant> {
    variant: Variant,
}

pub mod variant {

    use super::*;
    pub struct LmtpDispatch<C: Connector> {
        pub client: SmtpClient,
        pub connector: C,
    }
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

#[pin_project(project=LmtpStreamProj)]
pub enum LmtpStream<M: MailDataStream> {
    Ready(#[pin] M, bool),
    Closed(Result<M::Output>),
}

impl<M: MailDataStream> Write for LmtpStream<M>
where
    M::Error: std::error::Error + Send + Sync + 'static,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.project() {
            LmtpStreamProj::Ready(stream, false) => stream.poll_write(cx, buf),
            _ => Poll::Ready(Err(closed())),
        }
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.project() {
            LmtpStreamProj::Ready(stream, _) => stream.poll_flush(cx),
            LmtpStreamProj::Closed(_) => Poll::Ready(Err(closed())),
        }
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        loop {
            break match self.as_mut().project() {
                LmtpStreamProj::Ready(stream, closing @ false) => {
                    let len = ready!(stream.poll_write(cx, &[][..]))?;
                    debug_assert!(len == 0, "We just want the final dot");
                    *closing = true;
                    continue;
                }
                LmtpStreamProj::Ready(mut stream, true) => {
                    ready!(stream.as_mut().poll_close(cx))?;
                    let result = stream.result().map_err(|e| e.into());
                    self.set(LmtpStream::Closed(result));
                    continue;
                }
                LmtpStreamProj::Closed(_) => Poll::Ready(Err(closed())),
            };
        }
    }
}

fn closed() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::NotConnected,
        "The stream has already been closed.",
    )
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
