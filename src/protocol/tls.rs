use crate::common::*;
use async_tls::server::TlsStream;
use async_tls::Accept;
use async_tls::TlsAcceptor;
use std::ops::DerefMut;

pub trait IntoTlsCapableIO: Read + Write + Sized + Unpin {
    fn into_tls_capable<A: Into<Option<TlsAcceptor>>>(self, acceptor: A) -> TlsCapable<Self> {
        TlsCapable::new(self, acceptor.into())
    }
}
impl<IO: Read + Write + Unpin> IntoTlsCapableIO for IO {}

pub trait TlsCapableIO {
    fn start_tls(self: Pin<&mut Self>) -> Result<()>;
}

impl<T, TLSIO> TlsCapableIO for T
where
    T: DerefMut<Target = TLSIO> + Unpin,
    TLSIO: TlsCapableIO + Unpin,
{
    fn start_tls(mut self: Pin<&mut Self>) -> Result<()> {
        Pin::new(self.deref_mut()).start_tls()
    }
}

impl<IO: Read + Write + Unpin> TlsCapableIO for TlsCapable<IO> {
    fn start_tls(mut self: Pin<&mut Self>) -> Result<()> {
        match std::mem::replace(&mut *self, TlsCapable::Failed) {
            TlsCapable::PlainText(_) => Err("start_tls: TLS isnot enabled".into()),
            TlsCapable::Enabled(io, acceptor) => {
                trace!("Switching to TLS");
                // Calling `acceptor.accept` will start the TLS handshake
                // The handshake is a future we can await to get an encrypted
                // stream back.
                self.set(TlsCapable::HandShake(acceptor.clone().accept(io)));
                Ok(())
            }
            TlsCapable::HandShake(_) => Err("start_tls: TLS handshake already in progress".into()),
            TlsCapable::Encrypted(_) => Err("start_tls: TLS is already on".into()),
            TlsCapable::Failed => Err("start_tls: TLS setup failed".into()),
        }
    }
}

pub enum TlsCapable<IO> {
    PlainText(IO),
    Enabled(IO, TlsAcceptor),
    HandShake(Accept<IO>),
    Encrypted(TlsStream<IO>),
    Failed,
}

impl<IO: Read + Write + Unpin> TlsCapable<IO> {
    pub fn new(io: IO, acceptor: Option<TlsAcceptor>) -> Self {
        match acceptor {
            None => TlsCapable::PlainText(io),
            Some(acceptor) => TlsCapable::Enabled(io, acceptor),
        }
    }
    fn poll_tls(&mut self, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match std::mem::replace(self, TlsCapable::Failed) {
            TlsCapable::HandShake(mut handshake) => {
                trace!("Waiting for TLS handshake");
                match Pin::new(&mut handshake).poll(cx) {
                    Poll::Ready(Err(e)) => {
                        *self = TlsCapable::Failed;
                        Poll::Ready(Err(e.into()))
                    }
                    Poll::Pending => {
                        trace!("TLS is not ready");
                        *self = TlsCapable::HandShake(handshake);
                        Poll::Pending
                    }
                    Poll::Ready(Ok(stream)) => {
                        trace!("TLS is on!");
                        *self = TlsCapable::Encrypted(stream);
                        Poll::Ready(Ok(()))
                    }
                }
            }
            current @ TlsCapable::Enabled(_, _) => {
                *self = current;
                Poll::Ready(Ok(()))
            }
            current @ TlsCapable::PlainText(_) => {
                *self = current;
                Poll::Ready(Ok(()))
            }
            current @ TlsCapable::Encrypted(_) => {
                *self = current;
                Poll::Ready(Ok(()))
            }
            current @ TlsCapable::Failed => {
                *self = current;
                Self::ready_failed()
            }
        }
    }
    fn failed() -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Tls setup failed")
    }
    fn ready_failed<T>() -> Poll<std::io::Result<T>> {
        Poll::Ready(Err(Self::failed()))
    }
}
impl<IO> std::fmt::Debug for TlsCapable<IO> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        match self {
            TlsCapable::PlainText(_) => write!(f, "PlainText(...)"),
            TlsCapable::Enabled(_, _) => write!(f, "Enabled(...)"),
            TlsCapable::HandShake(_) => write!(f, "HandShake(...)"),
            TlsCapable::Encrypted(_) => write!(f, "Encrypted(...)"),
            TlsCapable::Failed => write!(f, "Failed"),
        }
    }
}

impl<IO: Read + Write + Unpin> Read for TlsCapable<IO> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::result::Result<usize, std::io::Error>> {
        ready!(self.poll_tls(cx))?;
        match &mut *self {
            TlsCapable::Encrypted(ref mut io) => Pin::new(io).poll_read(cx, buf),
            TlsCapable::PlainText(ref mut io) => Pin::new(io).poll_read(cx, buf),
            TlsCapable::Enabled(ref mut io, _) => Pin::new(io).poll_read(cx, buf),
            TlsCapable::HandShake(_) => unreachable!("this path is handled in poll_tls()"),
            TlsCapable::Failed => Self::ready_failed(),
        }
    }
}

impl<IO: Read + Write + Unpin> Write for TlsCapable<IO> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        ready!(self.poll_tls(cx))?;
        match &mut *self {
            TlsCapable::Encrypted(ref mut io) => Pin::new(io).poll_write(cx, buf),
            TlsCapable::PlainText(ref mut io) => Pin::new(io).poll_write(cx, buf),
            TlsCapable::Enabled(ref mut io, _) => Pin::new(io).poll_write(cx, buf),
            TlsCapable::HandShake(_) => unreachable!("this path is handled in poll_tls()"),
            TlsCapable::Failed => Self::ready_failed(),
        }
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        ready!(self.poll_tls(cx))?;
        match &mut *self {
            TlsCapable::Encrypted(ref mut io) => Pin::new(io).poll_flush(cx),
            TlsCapable::PlainText(ref mut io) => Pin::new(io).poll_flush(cx),
            TlsCapable::Enabled(ref mut io, _) => Pin::new(io).poll_flush(cx),
            TlsCapable::HandShake(_) => unreachable!("this path is handled in poll_tls()"),
            TlsCapable::Failed => Self::ready_failed(),
        }
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        ready!(self.poll_tls(cx))?;
        match &mut *self {
            TlsCapable::Encrypted(ref mut io) => Pin::new(io).poll_close(cx),
            TlsCapable::PlainText(ref mut io) => Pin::new(io).poll_close(cx),
            TlsCapable::Enabled(ref mut io, _) => Pin::new(io).poll_close(cx),
            TlsCapable::HandShake(_) => unreachable!("this path is handled in poll_tls()"),
            TlsCapable::Failed => Self::ready_failed(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::Result;
    use crate::test_util::*;
    use futures_await_test::async_test;

    #[async_test]
    async fn test_starttls_plaintext() -> Result<()> {
        let mut io = TestIO::from(b"STARTTLS\r\n");

        let mut tls = TlsCapable::new(&mut io, None);

        tls.write(b"220 OK\r\n").await?;

        assert_eq!(8, io.written().len());

        Ok(())
    }
}
