#[macro_use]
extern crate log;

pub use samotop_delivery::smtp::net;

use samotop_core::{
    common::*,
    model::mail::Transaction,
    model::mail::{DispatchError, DispatchResult},
    service::mail::{composite::*, *},
};
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

impl<ES: EsmtpService, GS: MailGuard, DS: MailDispatch, C: Connector> MailSetup<ES, GS, DS>
    for Config<variant::LmtpDispatch<C>>
where
    C: 'static,
{
    type Output = CompositeMailService<ES, GS, LmtpMail<Arc<SmtpTransport<SmtpClient, C>>>>;
    fn setup(self, extend: ES, guard: GS, _dispatch: DS) -> Self::Output {
        let transport = Arc::new(self.variant.client.connect_with(self.variant.connector));
        (extend, guard, LmtpMail::new(transport)).into()
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
    type Mail = LmtpStream<<SmtpTransport<SmtpClient, C> as Transport>::DataStream>;
    type MailFuture = Pin<Box<dyn Future<Output = DispatchResult<Self::Mail>> + Sync + Send>>;
    fn send_mail(&self, mail: Transaction) -> Self::MailFuture {
        let transport = self.inner.clone();
        let fut = future::ready((move || {
            let sender = mail
                .mail
                .map(|sender| EmailAddress::new(sender.from().address()))
                .transpose()?;
            let recipients: std::result::Result<Vec<_>, _> = mail
                .rcpts
                .iter()
                .map(|rcpt| EmailAddress::new(rcpt.address()))
                .collect();

            Envelope::new(sender, recipients?, mail.id).map_err(Error::from)
        })())
        .and_then(move |envelope| {
            trace!("Starting mail transaction.");
            send_stream(transport, envelope)
        })
        .and_then(|stream| {
            trace!("Starting mail data stream.");
            future::ready(Ok(LmtpStream::Ready(stream, false)))
        })
        .map_err(|e| {
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
