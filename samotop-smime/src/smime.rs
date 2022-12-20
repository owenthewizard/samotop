use async_macros::{join, ready};
use async_process::{Command, Stdio};
use async_std::io;
use log::*;
use samotop_core::common::Result;
use std::task::{Context, Poll};
use std::{future::Future, pin::Pin};

#[pin_project::pin_project]
pub struct SMime<'a> {
    #[pin]
    input: Option<Pin<Box<dyn io::Write + Sync + Send>>>,
    #[pin]
    copy: Pin<Box<dyn Future<Output = io::Result<()>> + Sync + Send + 'a>>,
}

impl<'a> SMime<'a> {
    pub fn encrypt_and_sign<W: io::Write + Sync + Send + 'a>(
        target: W,
        my_key: &str,
        my_cert: &str,
        her_cert: &str,
        their_certs: Vec<&str>,
    ) -> Result<Self> {
        Self::seal(target, my_key, my_cert, her_cert, their_certs, false)
    }
    pub fn sign_and_encrypt<W: io::Write + Sync + Send + 'a>(
        target: W,
        my_key: &str,
        my_cert: &str,
        her_cert: &str,
        their_certs: Vec<&str>,
    ) -> Result<Self> {
        Self::seal(target, my_key, my_cert, her_cert, their_certs, true)
    }
    pub fn decrypt_and_verify<W: io::Write + Sync + Send + 'a>(
        target: W,
        her_key: &str,
    ) -> Result<Self> {
        Self::open(target, her_key, true)
    }
    pub fn verify_and_decrypt<W: io::Write + Sync + Send + 'a>(
        target: W,
        her_key: &str,
    ) -> Result<Self> {
        Self::open(target, her_key, false)
    }
    fn seal<W: io::Write + Sync + Send + 'a>(
        target: W,
        my_key: &str,
        my_cert: &str,
        her_cert: &str,
        their_certs: Vec<&str>,
        sign_first: bool,
    ) -> Result<Self> {
        let mut sign = Command::new("openssl")
            .arg("smime")
            .arg("-stream")
            .arg("-sign")
            .arg("-inkey")
            .arg(my_key)
            .arg("-signer")
            .arg(my_cert)
            .kill_on_drop(true)
            .reap_on_drop(true)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let mut encrypt = Command::new("openssl");
        encrypt
            .arg("smime")
            .arg("-stream")
            .arg("-encrypt")
            .arg(her_cert);

        for crt in their_certs {
            encrypt.arg(crt);
        }

        let mut encrypt = encrypt
            .kill_on_drop(true)
            .reap_on_drop(true)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let sign_in = sign.stdin.take().expect("sign input");
        let sign_out = sign.stdout.take().expect("sign output");
        let encrypt_in = encrypt.stdin.take().expect("encrypt input");
        let encrypt_out = encrypt.stdout.take().expect("encrypt output");

        let (input, cp1, cp2) = if sign_first {
            (
                sign_in,
                CopyAndClose::new(sign_out, encrypt_in),
                CopyAndClose::new(encrypt_out, target),
            )
        } else {
            (
                encrypt_in,
                CopyAndClose::new(encrypt_out, sign_in),
                CopyAndClose::new(sign_out, target),
            )
        };

        let copy = async move {
            let (res1, res2) = join!(cp1, cp2).await;
            res1?;
            res2?;
            trace!("sign: {:?}", sign.status().await?);
            trace!("encrypt: {:?}", encrypt.status().await?);
            Ok(())
        };
        let writer = SMime {
            input: Some(Box::pin(input)),
            copy: Box::pin(copy),
        };

        Ok(writer)
    }

    fn open<W: io::Write + Sync + Send + 'a>(
        target: W,
        her_key: &str,
        sign_first: bool,
    ) -> Result<Self> {
        let mut verify = Command::new("openssl")
            .arg("smime")
            .arg("-stream")
            .arg("-verify")
            .arg("-noverify")
            .kill_on_drop(true)
            .reap_on_drop(true)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let mut decrypt = Command::new("openssl")
            .arg("smime")
            .arg("-stream")
            .arg("-decrypt")
            .arg("-inkey")
            .arg(her_key)
            .kill_on_drop(true)
            .reap_on_drop(true)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let verify_in = verify.stdin.take().expect("verify input");
        let verify_out = verify.stdout.take().expect("verify output");
        let decrypt_in = decrypt.stdin.take().expect("decrypt input");
        let decrypt_out = decrypt.stdout.take().expect("decrypt output");

        let (input, cp1, cp2) = if sign_first {
            (
                decrypt_in,
                CopyAndClose::new(decrypt_out, verify_in),
                CopyAndClose::new(verify_out, target),
            )
        } else {
            (
                verify_in,
                CopyAndClose::new(verify_out, decrypt_in),
                CopyAndClose::new(decrypt_out, target),
            )
        };

        let copy = async move {
            let (res1, res2) = join!(cp1, cp2).await;
            res1?;
            res2?;
            trace!("verify: {:?}", verify.status().await?);
            trace!("decrypt: {:?}", decrypt.status().await?);
            Ok(())
        };
        let writer = SMime {
            input: Some(Box::pin(input)),
            copy: Box::pin(copy),
        };

        Ok(writer)
    }
}

impl<'a> io::Write for SMime<'a> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.project();
        trace!("poll_write {}", buf.len());
        if let Some(i) = this.input.as_pin_mut() {
            trace!("poll_write input");
            let written = ready!(i.poll_write(cx, buf))?;
            Poll::Ready(Ok(written))
        } else {
            Poll::Ready(Err(io::ErrorKind::NotConnected.into()))
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut this = self.project();
        trace!("poll_flush");
        if let Some(i) = this.input.as_mut().as_pin_mut() {
            trace!("poll_flush input");
            ready!(i.poll_flush(cx))?;
        }
        trace!("flush poll copy...");
        if let Poll::Ready(copied) = this.copy.poll(cx) {
            // Copy is not meant to be ready.
            // It will finish when input is closed.
            // But we are moving it forward by polling here.
            warn!("flush poll copy => {:?}", copied);
        } else {
            trace!("flush poll copy => not ready");
        }
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut this = self.project();
        trace!("poll_close");
        if let Some(i) = this.input.as_mut().as_pin_mut() {
            trace!("poll_close input");
            ready!(i.poll_close(cx))?;
            // must drop input to finish processing
            this.input.set(None);
            trace!("closed input");
        }

        trace!("close poll copy...");
        ready!(this.copy.poll(cx))?;
        trace!("close poll copy");

        Poll::Ready(Ok(()))
    }
}

#[pin_project::pin_project]
struct CopyAndClose<R, W> {
    #[pin]
    reader: R,
    #[pin]
    writer: W,
    amt: u64,
}

impl<R, W> CopyAndClose<io::BufReader<R>, W>
where
    R: io::Read,
    W: io::Write,
{
    pub fn new(reader: R, writer: W) -> Self {
        CopyAndClose {
            reader: io::BufReader::new(reader),
            writer,
            amt: 0,
        }
    }
}

impl<R, W> async_std::future::Future for CopyAndClose<R, W>
where
    R: io::BufRead,
    W: io::Write,
{
    type Output = io::Result<u64>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        loop {
            trace!("copy poll_fill_buf...");
            let buffer = ready!(this.reader.as_mut().poll_fill_buf(cx))?;
            trace!("copy poll_fill_buf => {}", buffer.len());
            if buffer.is_empty() {
                trace!("copy poll_close...");
                ready!(this.writer.as_mut().poll_close(cx))?;
                return Poll::Ready(Ok(*this.amt));
            }

            trace!("copy poll_write...");
            let i = ready!(this.writer.as_mut().poll_write(cx, buffer))?;
            trace!("copy poll_write => {}", i);
            if i == 0 {
                return Poll::Ready(Err(io::ErrorKind::WriteZero.into()));
            }
            *this.amt += i as u64;
            this.reader.as_mut().consume(i);
        }
    }
}
