use crate::common::*;
use crate::service::tcp::tls::TlsProvider;
use crate::service::tcp::tls::TlsProviderFactory;
use std::ops::Deref;
use std::ops::DerefMut;

pub trait MayBeTls {
    fn can_encrypt(&self) -> bool;
    fn is_encrypted(&self) -> bool;
    fn start_tls(self: Pin<&mut Self>) -> std::io::Result<()>;
}

impl<T, TLSIO> MayBeTls for T
where
    T: DerefMut<Target = TLSIO> + Unpin,
    TLSIO: MayBeTls + Unpin,
{
    fn start_tls(mut self: Pin<&mut Self>) -> std::io::Result<()> {
        Pin::new(self.deref_mut()).start_tls()
    }
    fn can_encrypt(&self) -> bool {
        Deref::deref(self).can_encrypt()
    }
    fn is_encrypted(&self) -> bool {
        Deref::deref(self).is_encrypted()
    }
}

#[pin_project(project=Tls)]
pub enum TlsCapable<IO, P: TlsProvider<IO>> {
    PlainText(IO),
    Enabled(Option<IO>, P),
    HandShake(#[pin] P::UpgradeFuture),
    Encrypted(P::EncryptedIO),
    Failed,
}

impl<IO: Read + Write + Unpin, P: TlsProvider<IO>> MayBeTls for TlsCapable<IO, P> {
    fn start_tls(mut self: Pin<&mut Self>) -> std::io::Result<()> {
        match self.as_mut().project() {
            Tls::Enabled(io, provider) => {
                trace!("Switching to TLS");
                // Calling `upgrade_to_tls` will start the TLS handshake
                // The handshake is a future we can await to get an encrypted
                // stream back.
                let io = io.take().expect("start_tls: Workaround for Pin borrows");
                let handshake = TlsCapable::HandShake(provider.upgrade_to_tls(io));
                self.set(handshake);
                Ok(())
            }
            Tls::PlainText(_) => self.fail("start_tls: TLS is not enabled"),
            Tls::HandShake(_) => self.fail("start_tls: TLS handshake already in progress"),
            Tls::Encrypted(_) => self.fail("start_tls: TLS is already on"),
            Tls::Failed => self.fail("start_tls: TLS setup failed"),
        }
    }
    fn can_encrypt(&self) -> bool {
        match self {
            TlsCapable::PlainText(_) => false,
            TlsCapable::Enabled(_, _) => true,
            TlsCapable::HandShake(_) => false,
            TlsCapable::Encrypted(_) => false,
            TlsCapable::Failed => false,
        }
    }
    fn is_encrypted(&self) -> bool {
        match self {
            TlsCapable::PlainText(_) => false,
            TlsCapable::Enabled(_, _) => false,
            TlsCapable::HandShake(_) => true,
            TlsCapable::Encrypted(_) => true,
            TlsCapable::Failed => false,
        }
    }
}
impl<IO: Read + Write + Unpin, P: TlsProvider<IO>> TlsCapable<IO, P> {
    pub fn yes(io: IO, provider: P) -> Self {
        TlsCapable::new(io, Some(provider))
    }
    pub fn no(io: IO) -> Self {
        TlsCapable::new(io, None)
    }
    fn new(io: IO, provider: Option<P>) -> Self {
        match provider.into() {
            None => TlsCapable::PlainText(io),
            Some(provider) => TlsCapable::Enabled(Some(io), provider),
        }
    }
    fn poll_tls(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.as_mut().project() {
            Tls::HandShake(mut handshake) => {
                trace!("Waiting for TLS handshake");
                match Pin::new(&mut handshake).poll(cx) {
                    Poll::Ready(Err(e)) => {
                        self.set(TlsCapable::Failed);
                        Poll::Ready(Err(e))
                    }
                    Poll::Pending => {
                        trace!("TLS is not ready yet");
                        Poll::Pending
                    }
                    Poll::Ready(Ok(stream)) => {
                        trace!("TLS is on!");
                        self.set(TlsCapable::Encrypted(stream));
                        Poll::Ready(Ok(()))
                    }
                }
            }
            Tls::Enabled(_, _) | Tls::PlainText(_) | Tls::Encrypted(_) => Poll::Ready(Ok(())),
            Tls::Failed => Self::ready_failed(),
        }
    }
    fn fail<T>(mut self: Pin<&mut Self>, msg: &str) -> std::io::Result<T> {
        self.set(TlsCapable::Failed);
        Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, msg))
    }
    fn failed() -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Tls setup failed")
    }
    fn ready_failed<T>() -> Poll<std::io::Result<T>> {
        Poll::Ready(Err(Self::failed()))
    }
}

impl<IO: Read + Write + Unpin, P: TlsProvider<IO>> Read for TlsCapable<IO, P> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        ready!(self.as_mut().poll_tls(cx))?;
        match self.project() {
            Tls::Encrypted(ref mut io) => Pin::new(io).poll_read(cx, buf),
            Tls::PlainText(ref mut io) => Pin::new(io).poll_read(cx, buf),
            Tls::Enabled(Some(ref mut io), _) => Pin::new(io).poll_read(cx, buf),
            Tls::Enabled(None, _) => unreachable!("poll_read: Workaround for Pin borrows"),
            Tls::HandShake(_) => unreachable!("poll_read: This path is handled in poll_tls()"),
            Tls::Failed => Self::ready_failed(),
        }
    }
}

impl<IO: Read + Write + Unpin, P: TlsProvider<IO>> Write for TlsCapable<IO, P> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        ready!(self.as_mut().poll_tls(cx))?;
        match self.project() {
            Tls::Encrypted(ref mut io) => Pin::new(io).poll_write(cx, buf),
            Tls::PlainText(ref mut io) => Pin::new(io).poll_write(cx, buf),
            Tls::Enabled(Some(ref mut io), _) => Pin::new(io).poll_write(cx, buf),
            Tls::Enabled(None, _) => unreachable!("poll_write: Workaround for Pin borrows"),
            Tls::HandShake(_) => unreachable!("poll_write: This path is handled in poll_tls()"),
            Tls::Failed => Self::ready_failed(),
        }
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        ready!(self.as_mut().poll_tls(cx))?;
        match self.project() {
            Tls::Encrypted(ref mut io) => Pin::new(io).poll_flush(cx),
            Tls::PlainText(ref mut io) => Pin::new(io).poll_flush(cx),
            Tls::Enabled(Some(ref mut io), _) => Pin::new(io).poll_flush(cx),
            Tls::Enabled(None, _) => unreachable!("poll_flush: Workaround for Pin borrows"),
            Tls::HandShake(_) => unreachable!("poll_flush: This path is handled in poll_tls()"),
            Tls::Failed => Self::ready_failed(),
        }
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        ready!(self.as_mut().poll_tls(cx))?;
        match self.project() {
            Tls::Encrypted(ref mut io) => Pin::new(io).poll_close(cx),
            Tls::PlainText(ref mut io) => Pin::new(io).poll_close(cx),
            Tls::Enabled(Some(ref mut io), _) => Pin::new(io).poll_close(cx),
            Tls::Enabled(None, _) => unreachable!("poll_close: Workaround for Pin borrows"),
            Tls::HandShake(_) => unreachable!("poll_close: This path is handled in poll_tls()"),
            Tls::Failed => Self::ready_failed(),
        }
    }
}

impl<IO, P: TlsProvider<IO>> std::fmt::Debug for TlsCapable<IO, P> {
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
#[derive(Clone)]
pub struct TlsDisabled;

impl MayBeTls for TlsDisabled {
    fn start_tls(self: Pin<&mut Self>) -> std::io::Result<()> {
        unreachable!()
    }
    fn can_encrypt(&self) -> bool {
        false
    }
    fn is_encrypted(&self) -> bool {
        false
    }
}
impl<IO> TlsProviderFactory<IO> for TlsDisabled {
    type Provider = TlsDisabled;
    fn get(&self) -> Option<Self::Provider> {
        None
    }
}
impl<IO> TlsProvider<IO> for TlsDisabled {
    type EncryptedIO = TlsDisabled;
    type UpgradeFuture = future::Ready<std::io::Result<Self::EncryptedIO>>;
    fn upgrade_to_tls(&self, _io: IO) -> Self::UpgradeFuture {
        unreachable!()
    }
}
impl Read for TlsDisabled {
    fn poll_read(
        self: Pin<&mut Self>,
        __cx: &mut Context<'_>,
        __buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        unreachable!()
    }
}
impl Write for TlsDisabled {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        unreachable!()
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        unreachable!()
    }
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        unreachable!()
    }
}
