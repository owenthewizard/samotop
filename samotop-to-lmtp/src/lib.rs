#[macro_use]
extern crate log;

use async_smtp::prelude::{
    ClientSecurity, EmailAddress, Envelope as LmtpEnvelope, MailDataStream, SmtpClient,
    SmtpTransport, Transport,
};
use async_smtp::smtp::net::DefaultConnector;
use pin_project::pin_project;
use samotop_core::common::*;
use samotop_core::service::mail::composite::*;
use samotop_core::service::mail::*;
use std::marker::PhantomData;

pub struct Config<Variant> {
    phantom: PhantomData<Variant>,
}

pub mod variants {
    pub struct Delivery;
}

impl Config<variants::Delivery> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<NS: NamedService, ES: EsmtpService, GS: MailGuard, QS: MailQueue> MailSetup<NS, ES, GS, QS>
    for Config<variants::Delivery>
{
    type Output = CompositeMailService<NS, ES, GS, LmtpMail<variants::Delivery>>;
    fn setup(self, named: NS, extend: ES, guard: GS, _queue: QS) -> Self::Output {
        (named, extend, guard, LmtpMail::new(self)).into()
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
    Ready(M),
    Closing(M),
    Closed(Result<M::Output>),
}

impl<M: MailDataStream> Write for LmtpStream<M>
where
    M: Unpin,
    M::Error: std::error::Error + Send + Sync + 'static,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.project() {
            LmtpStreamProj::Ready(stream) => Pin::new(stream).poll_write(cx, buf),
            _ => Poll::Ready(Err(closed())),
        }
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.project() {
            LmtpStreamProj::Ready(stream) => Pin::new(stream).poll_flush(cx),
            _ => Poll::Ready(Err(closed())),
        }
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        loop {
            break match std::mem::replace(
                &mut *self,
                LmtpStream::Closed(Err("Broken state.".into())),
            ) {
                LmtpStream::Ready(mut stream) => {
                    match Pin::new(&mut stream).poll_write(cx, &[][..])? {
                        Poll::Pending => {
                            self.set(LmtpStream::Ready(stream));
                            Poll::Pending
                        }
                        Poll::Ready(len) => {
                            debug_assert!(len == 0, "We just want the final dot");
                            self.set(LmtpStream::Closing(stream));
                            continue;
                        }
                    }
                }
                LmtpStream::Closing(mut stream) => match Pin::new(&mut stream).poll_flush(cx)? {
                    Poll::Pending => {
                        self.set(LmtpStream::Closing(stream));
                        Poll::Pending
                    }
                    Poll::Ready(()) => {
                        self.set(LmtpStream::Closed(stream.result().map_err(|e| e.into())));
                        continue;
                    }
                },
                cur @ LmtpStream::Closed(_) => {
                    self.set(cur);
                    Poll::Ready(Err(closed()))
                }
            };
        }
    }
}

fn failed<E: std::fmt::Display + ?Sized>(err: &E) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::Other,
        format!("Sending mail failed: {}", err),
    )
}
fn broken() -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, "The stream is broken.")
}
fn closed() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::NotConnected,
        "The stream has already been closed.",
    )
}

impl<Any> MailQueue for LmtpMail<Any> {
    type Mail = LmtpStream<<SmtpTransport<SmtpClient, DefaultConnector> as Transport>::DataStream>;
    type MailFuture = Pin<Box<dyn Future<Output = Option<Self::Mail>> + Sync + Send>>;
    fn mail(&self, mail: samotop_core::model::mail::Envelope) -> Self::MailFuture {
        let fut = async move {
            let sender = mail
                .mail
                .map(|sender| EmailAddress::new(sender.from().to_string()))
                .transpose()?;
            let recipients: std::result::Result<Vec<_>, _> = mail
                .rcpts
                .iter()
                .map(|rcpt| EmailAddress::new(rcpt.to_string()))
                .collect();

            let envelope = LmtpEnvelope::new(sender, recipients?, mail.id)?;
            let stream = SmtpClient::with_security("localhost:2525", ClientSecurity::None)?
                .connect_and_send_stream(envelope)
                .await?;
            Ok(LmtpStream::Ready(stream))
        }
        .then(|res: Result<_>| match res {
            Ok(stream) => {
                trace!("Starting mail.");
                future::ready(Some(stream))
            }
            Err(e) => {
                error!("Failed to start mail: {:?}", e);
                future::ready(None)
            }
        });
        //unimplemented!("Future is not Sync");
        Box::pin(fut)
    }
    fn new_id(&self) -> String {
        unimplemented!()
    }
}
