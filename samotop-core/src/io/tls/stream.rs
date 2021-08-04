use super::{Io, MayBeTls, TlsUpgrade};
use crate::common::*;
use core::panic;
use std::fmt;

pub struct TlsCapable {
    state: State,
}

enum State {
    /// TLS upgrade is not enabled - only plaintext or wrapper mode
    /// or is already encrypted
    Done(Box<dyn Io>, bool),
    /// Plain TCP stream with name and potential TLS upgrade
    Enabled(Box<dyn Io>, Box<dyn TlsUpgrade>, String),
    /// Pending TLS handshake
    Handshake(S3Fut<std::io::Result<Box<dyn Io>>>),
    /// TLS failed or in transition state
    Failed,
}

impl MayBeTls for TlsCapable {
    fn encrypt(mut self: Pin<&mut Self>) {
        match std::mem::replace(&mut self.state, State::Failed) {
            State::Enabled(io, provider, peer_name) => {
                trace!("Switching to TLS");
                // Calling `upgrade_to_tls` will start the TLS handshake
                // The handshake is a future we can await to get an encrypted
                // stream back.
                let newme = State::Handshake(Box::pin(provider.upgrade_to_tls(io, peer_name)));
                self.state = newme;
            }
            State::Done(_, encrypted) => self.fail(
                format!(
                    "start_tls: TLS upgrade is not enabled. encrypted: {}",
                    encrypted,
                )
                .as_str(),
            ),
            State::Handshake(_) => self.fail("start_tls: TLS handshake already in progress"),
            State::Failed => self.fail("start_tls: TLS setup failed"),
        }
    }
    fn can_encrypt(&self) -> bool {
        match self.state {
            State::Done(_, _) => false,
            State::Enabled(_, _, _) => true,
            State::Handshake(_) => false,
            State::Failed => false,
        }
    }
    fn is_encrypted(&self) -> bool {
        match self.state {
            State::Done(_, encrypted) => encrypted,
            State::Enabled(_, _, _) => false,
            State::Handshake(_) => true,
            State::Failed => false,
        }
    }

    fn enable_encryption(&mut self, upgrade: Box<dyn super::TlsUpgrade>, name: String) {
        self.state = match std::mem::replace(&mut self.state, State::Failed) {
            State::Enabled(io, _, _) => State::Enabled(io, upgrade, name),
            State::Done(io, _) => State::Enabled(io, upgrade, name),
            State::Handshake(_) => panic!("currently upgrading"),
            State::Failed => panic!("IO failed"),
        }
    }
}
impl TlsCapable {
    pub fn plaintext(io: Box<dyn Io>) -> Self {
        TlsCapable {
            state: State::Done(io, false),
        }
    }
    pub fn encrypted(io: Box<dyn Io>) -> Self {
        TlsCapable {
            state: State::Done(io, true),
        }
    }
    pub fn enabled(io: Box<dyn Io>, upgrade: Box<dyn TlsUpgrade>, peer_name: String) -> Self {
        TlsCapable {
            state: State::Enabled(io, upgrade, peer_name),
        }
    }
    fn poll_tls(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        match this.state {
            State::Handshake(ref mut h) => {
                trace!("Waiting for TLS handshake");
                match Pin::new(h).poll(cx)? {
                    Poll::Pending => {
                        trace!("TLS is not ready yet");
                        Poll::Pending
                    }
                    Poll::Ready(encrypted) => {
                        trace!("TLS is on!");
                        this.state = State::Done(encrypted, true);
                        Poll::Ready(Ok(()))
                    }
                }
            }
            _ => Poll::Ready(Ok(())),
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

impl io::Read for TlsCapable {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        trace!("poll_read on {:?}", self.state);
        match (self.as_mut().poll_tls(cx))? {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(()) => (),
        };
        let result = match self.state {
            State::Done(ref mut io, _) => Pin::new(io).poll_read(cx, buf),
            State::Enabled(ref mut io, _, _) => Pin::new(io).poll_read(cx, buf),
            State::Handshake(_) => unreachable!("poll_read: This path is handled in poll_tls()"),
            State::Failed => Self::ready_failed(),
        };
        trace!("poll_read got {:?}", result);
        result
    }
}

impl io::Write for TlsCapable {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match (self.as_mut().poll_tls(cx))? {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(()) => (),
        };
        match self.state {
            State::Done(ref mut io, _) => Pin::new(io).poll_write(cx, buf),
            State::Enabled(ref mut io, _, _) => Pin::new(io).poll_write(cx, buf),
            State::Handshake(_) => unreachable!("poll_write: This path is handled in poll_tls()"),
            State::Failed => Self::ready_failed(),
        }
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match (self.as_mut().poll_tls(cx))? {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(()) => (),
        };
        match self.state {
            State::Done(ref mut io, _) => Pin::new(io).poll_flush(cx),
            State::Enabled(ref mut io, _, _) => Pin::new(io).poll_flush(cx),
            State::Handshake(_) => unreachable!("poll_flush: This path is handled in poll_tls()"),
            State::Failed => Self::ready_failed(),
        }
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match (self.as_mut().poll_tls(cx))? {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(()) => (),
        };
        match self.state {
            State::Done(ref mut io, _) => Pin::new(io).poll_close(cx),
            State::Enabled(ref mut io, _, _) => Pin::new(io).poll_close(cx),
            State::Handshake(_) => unreachable!("poll_close: This path is handled in poll_tls()"),
            State::Failed => Self::ready_failed(),
        }
    }
}

impl std::fmt::Debug for TlsCapable {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        self.state.fmt(f)
    }
}

impl fmt::Debug for State {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use State as S;
        fmt.write_str(match self {
            S::Done(_, encrypted) => {
                if *encrypted {
                    "Done(stream,encrypted)"
                } else {
                    "Done(stream,plaintext)"
                }
            }
            S::Enabled(_, _, _) => "Enabled(stream, upgrade, peer_name)",
            S::Handshake(_) => "Handshake(tls_handshake)",
            S::Failed => "Failed",
        })
    }
}
