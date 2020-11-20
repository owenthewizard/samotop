//! Traits and impls to represent and establish network-like streams
pub mod tls;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use self::unix::*;

mod inet;
pub use self::inet::*;

use self::tls::{DefaultTls, TlsUpgrade};
use crate::smtp::extension::ClientId;
use crate::smtp::extension::ServerInfo;
use crate::ClientSecurity;
use crate::{smtp::authentication::Authentication, SyncFuture};
use async_std::io::{self, Read, Write};
use async_std::net::SocketAddr;
use async_std::pin::Pin;
use async_std::task::{Context, Poll};
use futures::{ready, Future};
use pin_project::pin_project;
use samotop_model::io::MayBeTls;
use std::fmt;
use std::time::Duration;

pub trait Connector: fmt::Debug + Sync + Send {
    type Stream: MayBeTls + Read + Write + Unpin + Sync + Send + 'static;
    /// This provider of connectivity takes care of resolving
    /// given address (which could be an IP, FQDN, URL...),
    /// establishing a connection and enabling (or not) TLS upgrade.

    fn connect<'s, 'c, 'a, C: ConnectionConfiguration>(
        &'s self,
        configuration: &'c C,
    ) -> SyncFuture<'a, io::Result<Self::Stream>>
    where
        's: 'a,
        'c: 'a;
}

pub trait ConnectionConfiguration: Sync + Send {
    fn address(&self) -> String;
    fn timeout(&self) -> Duration;
    fn security(&self) -> ClientSecurity;
    fn hello_name(&self) -> ClientId;
    fn max_reuse_count(&self) -> u16;
    fn get_authentication(
        &self,
        server_info: &ServerInfo,
        encrypted: bool,
    ) -> Option<Box<dyn Authentication>>;
    fn lmtp(&self) -> bool;
}

pub type DefaultConnector = inet::TcpConnector<DefaultTls>;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TlsMode {
    Tls,
    StartTls,
}

#[pin_project(project = NetStreamProj)]
#[derive(Debug)]
pub struct NetworkStream<S, E, U> {
    state: State<S, E, U>,
    peer_addr: Option<SocketAddr>,
    peer_name: String,
}

/// Represents the different types of underlying network streams
#[pin_project(project = StateProj)]
#[allow(missing_debug_implementations)]
enum State<S, E, U> {
    /// TLS upgrade is not enabled
    Disabled(#[pin] S),
    /// Plain TCP stream with name and potential TLS upgrade
    Enabled(#[pin] S, U),
    /// Encrypted TCP stream
    Encrypted(#[pin] E),
    /// Pending TLS handshake
    Handshake(Pin<Box<dyn Future<Output = io::Result<E>> + Sync + Send>>),
    /// Transitional state to help take owned values from the enum
    /// Invalid outside of an &mut own method call/future
    Failed,
}

impl<S, U> MayBeTls for NetworkStream<S, U::Encrypted, U>
where
    U: TlsUpgrade<S>,
{
    /// Initiates the TLS negotiations.
    /// The stream must then block all reads/writes until the
    /// underlying TLS handshake is done.
    fn encrypt(mut self: Pin<&mut Self>) {
        match std::mem::replace(&mut self.state, State::Failed) {
            State::Enabled(stream, upgrade) => {
                self.state = State::Handshake(Box::pin(
                    upgrade.upgrade_to_tls(stream, self.peer_name.clone()),
                ));
            }
            otherwise => {
                error!("Invalid state to encrypt now: {:?}", otherwise);
                self.state = State::Failed;
            }
        }
    }
    /// Returns true only if calling encrypt would make sense:
    /// 1. required encryption setup information is available.
    /// 2. the stream is not encrypted yet.
    fn can_encrypt(&self) -> bool {
        match self.state {
            State::Enabled(_, _) => true,
            State::Disabled(_) => false,
            State::Encrypted(_) => false,
            State::Handshake(_) => false,
            State::Failed => false,
        }
    }
    /// Returns true if the stream is already encrypted (or hand shaking).
    fn is_encrypted(&self) -> bool {
        match self.state {
            State::Enabled(_, _) => false,
            State::Disabled(_) => false,
            State::Encrypted(_) => true,
            State::Handshake(_) => true,
            State::Failed => false,
        }
    }
}

impl<S, E, U> NetworkStream<S, E, U> {
    fn poll_tls(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        let proj = self.project();
        match std::mem::replace(proj.state, State::Failed) {
            State::Handshake(mut h) => match Pin::new(&mut h).poll(cx)? {
                Poll::Pending => {
                    *proj.state = State::Handshake(h);
                    Poll::Pending
                }
                Poll::Ready(encrypted) => {
                    *proj.state = State::Encrypted(encrypted);
                    Poll::Ready(Ok(()))
                }
            },
            otherwise => {
                *proj.state = otherwise;
                Poll::Ready(Ok(()))
            }
        }
    }
}

impl<S, E, U> Read for NetworkStream<S, E, U>
where
    S: Read + Unpin,
    E: Read + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        trace!("poll_read with {:?}", self.state);
        ready!(self.as_mut().poll_tls(cx))?;
        let result = match self.state {
            State::Disabled(ref mut s) => Pin::new(s).poll_read(cx, buf),
            State::Enabled(ref mut s, _) => Pin::new(s).poll_read(cx, buf),
            State::Encrypted(ref mut s) => Pin::new(s).poll_read(cx, buf),
            State::Handshake(_) => {
                unreachable!("Handshake is handled by poll_tls");
            }
            State::Failed => Poll::Ready(Err(broken())),
        };
        trace!("poll_read got {:?}", result);
        result
    }
}

impl<S, E, U> Write for NetworkStream<S, E, U>
where
    S: Write + Unpin,
    E: Write + Unpin,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        ready!(self.as_mut().poll_tls(cx))?;
        match self.state {
            State::Disabled(ref mut s) => Pin::new(s).poll_write(cx, buf),
            State::Enabled(ref mut s, _) => Pin::new(s).poll_write(cx, buf),
            State::Encrypted(ref mut s) => Pin::new(s).poll_write(cx, buf),
            State::Handshake(_) => {
                unreachable!("Handshake is handled by poll_tls");
            }
            State::Failed => Poll::Ready(Err(broken())),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        ready!(self.as_mut().poll_tls(cx))?;
        match self.state {
            State::Disabled(ref mut s) => Pin::new(s).poll_flush(cx),
            State::Enabled(ref mut s, _) => Pin::new(s).poll_flush(cx),
            State::Encrypted(ref mut s) => Pin::new(s).poll_flush(cx),
            State::Handshake(_) => {
                unreachable!("Handshake is handled by poll_tls");
            }
            State::Failed => Poll::Ready(Err(broken())),
        }
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        ready!(self.as_mut().poll_tls(cx))?;
        match self.state {
            State::Disabled(ref mut s) => Pin::new(s).poll_close(cx),
            State::Enabled(ref mut s, _) => Pin::new(s).poll_close(cx),
            State::Encrypted(ref mut s) => Pin::new(s).poll_close(cx),
            State::Handshake(_) => {
                unreachable!("Handshake is handled by poll_tls");
            }
            State::Failed => Poll::Ready(Err(broken())),
        }
    }
}

fn broken() -> io::Error {
    io::Error::new(io::ErrorKind::BrokenPipe, "Invalid network stream state")
}

impl<S, E, U> fmt::Debug for State<S, E, U> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        use State::*;
        fmt.write_str(match self {
            Disabled(_) => "Disabled(stream)",
            Enabled(_, _) => "Enabled(stream, upgrade)",
            Encrypted(_) => "Encrypted(tls_stream)",
            Handshake(_) => "Handshake(tls_handshake)",
            Failed => "Failed",
        })
    }
}
