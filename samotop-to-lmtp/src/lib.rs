#[macro_use]
extern crate log;

mod net;

use crate::net::*;
use async_smtp::prelude::{
    EmailAddress, Envelope, MailDataStream, SmtpClient, SmtpTransport, Transport,
};
use samotop_core::common::*;
use samotop_core::model::mail::DispatchError;
use samotop_core::model::mail::DispatchResult;
use samotop_core::model::mail::Transaction;
use samotop_core::service::mail::composite::*;
use samotop_core::service::mail::*;
use std::marker::PhantomData;

pub struct Config<Variant> {
    phantom: PhantomData<Variant>,
    address: String,
}

pub mod variants {
    pub struct Delivery;
}

impl Config<variants::Delivery> {
    pub fn new(address: String) -> Self {
        Self {
            phantom: PhantomData,
            address,
        }
    }
}

impl<ES: EsmtpService, GS: MailGuard, DS: MailDispatch> MailSetup<ES, GS, DS>
    for Config<variants::Delivery>
{
    type Output = CompositeMailService<ES, GS, LmtpMail<variants::Delivery>>;
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
            LmtpStreamProj::Closing(stream) => Pin::new(stream).poll_flush(cx),
            LmtpStreamProj::Closed(_) => Poll::Ready(Err(closed())),
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

fn closed() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::NotConnected,
        "The stream has already been closed.",
    )
}

impl<Any> MailDispatch for LmtpMail<Any> {
    type Mail = LmtpStream<<SmtpTransport<SmtpClient, MyCon> as Transport>::DataStream>;
    type MailFuture = Pin<Box<dyn Future<Output = DispatchResult<Self::Mail>> + Sync + Send>>;
    fn send_mail(&self, mail: Transaction) -> Self::MailFuture {
        let addr = self.config.address.clone();
        let connector = conn();
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

            let envelope = Envelope::new(sender, recipients?, mail.id)?;
            let stream = SmtpClient::new(addr)?
                .connect_with(connector)
                .await?
                .send_stream(envelope)
                .await?;
            Ok(LmtpStream::Ready(stream))
        }
        .then(|res: Result<_>| match res {
            Ok(stream) => {
                trace!("Starting mail.");
                future::ready(Ok(stream))
            }
            Err(e) => {
                error!("Failed to start mail: {:?}", e);
                future::ready(Err(DispatchError::FailedTemporarily))
            }
        });
        Box::pin(fut)
    }
}
