use std::fmt;

use crate::common::*;
use crate::io::tls::{TlsProvider, TlsProviderFactory};
use crate::io::*;

#[pin_project(project=TlsCapProj)]
pub struct TlsCapable<IO, P: TlsProvider<IO>> {
    state: State<IO, P>,
}

enum State<IO, P: TlsProvider<IO>> {
    /// TLS upgrade is not enabled, only plaintext or wrapper mode
    PlainText(IO),
    /// Plain TCP stream with name and potential TLS upgrade
    Enabled(IO, P),
    /// Pending TLS handshake
    Handshake(Pin<Box<dyn Future<Output = std::io::Result<P::EncryptedIO>> + Sync + Send>>),
    /// Encrypted TCP stream
    Encrypted(P::EncryptedIO),
    /// TLS failed or in transition state
    Failed,
}

impl<IO: Read + Write + Unpin, P: TlsProvider<IO>> MayBeTls for TlsCapable<IO, P> {
    fn encrypt(mut self: Pin<&mut Self>) {
        match std::mem::replace(&mut self.state, State::Failed) {
            State::Enabled(io, provider) => {
                trace!("Switching to TLS");
                // Calling `upgrade_to_tls` will start the TLS handshake
                // The handshake is a future we can await to get an encrypted
                // stream back.
                let newme = State::Handshake(Box::pin(provider.upgrade_to_tls(io)));
                self.state = newme;
            }
            State::PlainText(_) => self.fail("start_tls: TLS is not enabled"),
            State::Handshake(_) => self.fail("start_tls: TLS handshake already in progress"),
            State::Encrypted(_) => self.fail("start_tls: TLS is already on"),
            State::Failed => self.fail("start_tls: TLS setup failed"),
        }
    }
    fn can_encrypt(&self) -> bool {
        match self.state {
            State::PlainText(_) => false,
            State::Enabled(_, _) => true,
            State::Handshake(_) => false,
            State::Encrypted(_) => false,
            State::Failed => false,
        }
    }
    fn is_encrypted(&self) -> bool {
        match self.state {
            State::PlainText(_) => false,
            State::Enabled(_, _) => false,
            State::Handshake(_) => true,
            State::Encrypted(_) => true,
            State::Failed => false,
        }
    }
}
impl<IO: Read + Write + Unpin> TlsCapable<IO, TlsDisabled> {
    pub fn disabled(io: IO) -> Self {
        TlsCapable {
            state: State::PlainText(io),
        }
    }
}
impl<IO: Read + Write + Unpin, P: TlsProvider<IO>> TlsCapable<IO, P> {
    pub fn yes(io: IO, provider: P) -> Self {
        TlsCapable {
            state: State::Enabled(io, provider),
        }
    }
    pub fn no(io: IO) -> Self {
        TlsCapable {
            state: State::PlainText(io),
        }
    }
    fn poll_tls(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match &mut self.state {
            State::Handshake(ref mut h) => {
                trace!("Waiting for TLS handshake");
                match Pin::new(h).poll(cx)? {
                    Poll::Pending => {
                        trace!("TLS is not ready yet");
                        Poll::Pending
                    }
                    Poll::Ready(encrypted) => {
                        trace!("TLS is on!");
                        self.state = State::Encrypted(encrypted);
                        Poll::Ready(Ok(()))
                    }
                }
            }
            _otherwise => Poll::Ready(Ok(())),
        }
    }
    fn fail(mut self: Pin<&mut Self>, msg: &str) {
        error!("{}", msg);
        self.state = State::Failed;
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
        trace!("poll_read on {:?}", self.state);
        ready!(self.as_mut().poll_tls(cx))?;
        let result = match self.state {
            State::Encrypted(ref mut io) => Pin::new(io).poll_read(cx, buf),
            State::PlainText(ref mut io) => Pin::new(io).poll_read(cx, buf),
            State::Enabled(ref mut io, _) => Pin::new(io).poll_read(cx, buf),
            State::Handshake(_) => unreachable!("poll_read: This path is handled in poll_tls()"),
            State::Failed => Self::ready_failed(),
        };
        trace!("poll_read got {:?}", result);
        result
    }
}

impl<IO: Read + Write + Unpin, P: TlsProvider<IO>> Write for TlsCapable<IO, P> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        ready!(self.as_mut().poll_tls(cx))?;
        match self.state {
            State::Encrypted(ref mut io) => Pin::new(io).poll_write(cx, buf),
            State::PlainText(ref mut io) => Pin::new(io).poll_write(cx, buf),
            State::Enabled(ref mut io, _) => Pin::new(io).poll_write(cx, buf),
            State::Handshake(_) => unreachable!("poll_write: This path is handled in poll_tls()"),
            State::Failed => Self::ready_failed(),
        }
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        ready!(self.as_mut().poll_tls(cx))?;
        match self.state {
            State::Encrypted(ref mut io) => Pin::new(io).poll_flush(cx),
            State::PlainText(ref mut io) => Pin::new(io).poll_flush(cx),
            State::Enabled(ref mut io, _) => Pin::new(io).poll_flush(cx),
            State::Handshake(_) => unreachable!("poll_flush: This path is handled in poll_tls()"),
            State::Failed => Self::ready_failed(),
        }
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        ready!(self.as_mut().poll_tls(cx))?;
        match self.state {
            State::Encrypted(ref mut io) => Pin::new(io).poll_close(cx),
            State::PlainText(ref mut io) => Pin::new(io).poll_close(cx),
            State::Enabled(ref mut io, _) => Pin::new(io).poll_close(cx),
            State::Handshake(_) => unreachable!("poll_close: This path is handled in poll_tls()"),
            State::Failed => Self::ready_failed(),
        }
    }
}

impl<IO, P: TlsProvider<IO>> std::fmt::Debug for TlsCapable<IO, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        self.state.fmt(f)
    }
}

impl<IO, P: TlsProvider<IO>> fmt::Debug for State<IO, P> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use State as S;
        fmt.write_str(match self {
            S::PlainText(_) => "PlainText(stream)",
            S::Enabled(_, _) => "Enabled(stream, upgrade)",
            S::Encrypted(_) => "Encrypted(tls_stream)",
            S::Handshake(_) => "Handshake(tls_handshake)",
            S::Failed => "Failed",
        })
    }
}

#[derive(Clone)]
pub struct TlsDisabled;

impl<IO> TlsProviderFactory<IO> for TlsDisabled {
    type Provider = TlsDisabled;
    fn get(&self) -> Option<Self::Provider> {
        None
    }
}
impl<IO> TlsProvider<IO> for TlsDisabled {
    type EncryptedIO = TlsDisabled;
    fn upgrade_to_tls(&self, _io: IO) -> S3Fut<std::io::Result<Self::EncryptedIO>> {
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
