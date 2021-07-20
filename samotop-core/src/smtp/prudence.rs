use crate::common::*;
use crate::io::tls::MayBeTls;
use crate::mail::{EsmtpService, MailSetup};
use crate::smtp::SmtpState;
use async_std::prelude::FutureExt;
use std::time::Duration;

/// Prevent or monitor bad SMTP behavior
#[derive(Debug)]
pub struct Prudence {
    /// Monitor bad behavior of clients not waiting for a banner given time
    pub wait_for_banner_delay: Option<Duration>,
}

impl MailSetup for Prudence {
    fn setup(self, config: &mut crate::mail::Configuration) {
        config.esmtp.insert(0, Box::new(self));
    }
}

impl EsmtpService for Prudence {
    fn prepare_session<'a, 'i, 's, 'f>(
        &'a self,
        io: &'i mut Box<dyn MayBeTls>,
        state: &'s mut SmtpState,
    ) -> crate::common::S1Fut<'f, ()>
    where
        'a: 'f,
        'i: 'f,
        's: 'f,
    {
        Box::pin(async move {
            if !state.session.banner_sent {
                if let Some(delay) = self.wait_for_banner_delay {
                    let mut buf = [0u8];
                    match io
                        .read(&mut buf[..])
                        .timeout(delay)
                        .await
                        .ok(/*convert timeout result to option*/)
                    {
                        Some(Ok(0)) => {
                            // this just looks like the client gave up and left
                        }
                        Some(Ok(_)) => {
                            let myio = std::mem::replace(io, Box::new(Dummy));
                            *io = Box::new(ConcatRW {
                                head: Some(buf[0]),
                                io: myio,
                            });

                            state.session.banner_sent = true;
                            state.say_shutdown_processing_err(
                                "Client sent commands before banner".into(),
                            );
                        }
                        Some(Err(e)) => {
                            state.session.banner_sent = true;
                            state.say_shutdown_processing_err(format!("IO read failed {}", e));
                        }
                        None => {
                            // timeout is correct behavior, well done!
                        }
                    }
                }
            }
        })
    }
}

struct ConcatRW {
    head: Option<u8>,
    io: Box<dyn MayBeTls>,
}

impl MayBeTls for ConcatRW {
    fn enable_encryption(&mut self, upgrade: Box<dyn crate::io::tls::TlsUpgrade>, name: String) {
        if self.head.is_some() {
            panic!("Cannot enable encryption while there are unread bytes in buffer")
        }
        self.io.enable_encryption(upgrade, name)
    }

    fn encrypt(mut self: Pin<&mut Self>) {
        if self.head.is_some() {
            panic!("Cannot encrypt while there are unread bytes in buffer")
        }
        Pin::new(&mut self.io).encrypt()
    }

    fn can_encrypt(&self) -> bool {
        self.head.is_none() && self.io.can_encrypt()
    }

    fn is_encrypted(&self) -> bool {
        self.head.is_none() && self.io.is_encrypted()
    }
}

impl Read for ConcatRW {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        if !buf.is_empty() {
            if let Some(b) = self.head.take() {
                buf[0] = b;
                return Poll::Ready(Ok(1));
            }
        }
        Pin::new(&mut self.io).poll_read(cx, buf)
    }
}
impl Write for ConcatRW {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.io).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.io).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.io).poll_close(cx)
    }
}
