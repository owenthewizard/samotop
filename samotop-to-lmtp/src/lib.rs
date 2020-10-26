#[macro_use]
extern crate log;

mod net;

use crate::net::*;
use samotop_core::common::*;
use samotop_core::model::mail::DispatchError;
use samotop_core::model::mail::DispatchResult;
use samotop_core::model::mail::Transaction;
use samotop_core::service::mail::composite::*;
use samotop_core::service::mail::*;
use samotop_delivery::prelude::{
    EmailAddress, Envelope, MailDataStream, SmtpClient, SmtpTransport, Transport,
};
use samotop_delivery::smtp::ConnectionReuseParameters;

pub struct Config<Variant> {
    variant: Variant,
    address: String,
}

pub mod variant {
    use super::*;
    pub struct TcpLmtpDispatch {
        pub transport: Arc<SmtpTransport<SmtpClient, MyCon>>,
    }
}

impl Config<variant::TcpLmtpDispatch> {
    pub fn tcp_lmtp_dispatch(address: String) -> Result<Self> {
        let variant = variant::TcpLmtpDispatch {
            transport: Arc::new(
                SmtpClient::new(&address)?
                    .connection_reuse(ConnectionReuseParameters::ReuseUnlimited)
                    .connect_with(conn()),
            ),
        };
        Ok(Self { variant, address })
    }
}

impl<ES: EsmtpService, GS: MailGuard, DS: MailDispatch> MailSetup<ES, GS, DS>
    for Config<variant::TcpLmtpDispatch>
{
    type Output = CompositeMailService<ES, GS, LmtpMail<variant::TcpLmtpDispatch>>;
    fn setup(self, extend: ES, guard: GS, _dispatch: DS) -> Self::Output {
        (extend, guard, LmtpMail::new(self)).into()
    }
}

pub struct LmtpMail<Variant> {
    config: Config<Variant>,
}

impl<Any> LmtpMail<Any> {
    fn new(config: Config<Any>) -> Self {
        Self { config }
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
                    ready!(stream.as_mut().poll_flush(cx))?;
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

impl MailDispatch for LmtpMail<variant::TcpLmtpDispatch> {
    type Mail = LmtpStream<<SmtpTransport<SmtpClient, MyCon> as Transport>::DataStream>;
    type MailFuture = Pin<Box<dyn Future<Output = DispatchResult<Self::Mail>> + Sync + Send>>;
    fn send_mail(&self, mail: Transaction) -> Self::MailFuture {
        let transport = self.config.variant.transport.clone();
        let fut = future::ready((move || {
            let sender = mail
                .mail
                .map(|sender| EmailAddress::new(sender.from().to_string()))
                .transpose()?;
            let recipients: std::result::Result<Vec<_>, _> = mail
                .rcpts
                .iter()
                .map(|rcpt| EmailAddress::new(rcpt.to_string()))
                .collect();

            Envelope::new(sender, recipients?, mail.id).map_err(|e| Error::from(e))
        })())
        .and_then(move |envelope| send_stream(transport, envelope))
        .and_then(|stream| {
            trace!("Starting mail.");
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
async fn send_stream(
    transport: Arc<SmtpTransport<SmtpClient, MyCon>>,
    envelope: Envelope,
) -> Result<<SmtpTransport<SmtpClient, MyCon> as Transport>::DataStream> {
    Ok(transport.send_stream(envelope).await?)
}
