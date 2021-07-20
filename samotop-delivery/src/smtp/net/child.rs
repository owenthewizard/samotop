use crate::smtp::net::tls::TlsCapable;
use crate::smtp::net::tls::TlsProvider;
use crate::smtp::net::Connector;
use crate::smtp::net::TlsMode;
use crate::{smtp::net::ConnectionConfiguration, SyncFuture};
use async_std::io;
use async_std::io::Read;
use async_std::io::Write;
use async_std::process::*;
use async_std::task::ready;
use samotop_core::common::Pin;
use samotop_core::io::tls::MayBeTls;
use std::fmt::Display;
use std::future::Future;
use std::task::Poll;

/// Allows the SMTP client to spawn a child process and connect to it's IO
#[derive(Debug)]
pub struct ChildConnector<TLS> {
    pub tls_mode: TlsMode,
    pub provider: TLS,
}

impl<TLS: Default> Default for ChildConnector<TLS> {
    fn default() -> Self {
        Self {
            tls_mode: TlsMode::StartTls,
            provider: TLS::default(),
        }
    }
}

impl<TLS> Connector for ChildConnector<TLS>
where
    TLS: TlsProvider + Sync + Send + 'static,
{
    type Stream = TlsCapable;
    /// This provider of connectivity takes care of running
    /// the program given in address (which should be an executable command),
    /// establishing a connection and enabling (or not) TLS upgrade.
    fn connect<'s, 'c, 'a, C: ConnectionConfiguration + Sync>(
        &'s self,
        configuration: &'c C,
    ) -> SyncFuture<'a, io::Result<Self::Stream>>
    where
        's: 'a,
        'c: 'a,
    {
        Box::pin(async move {
            let to = configuration.address();

            let child = Command::new(&to)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true)
                .reap_on_drop(true)
                .spawn()?;

            let stream = Box::new(ChildIo {
                inner: Some(child),
                closing: None,
            });
            let mut stream = match self.provider.get_tls_upgrade() {
                Some(u) => TlsCapable::enabled(stream, u, to),
                None => TlsCapable::plaintext(stream),
            };

            match self.tls_mode {
                TlsMode::Tls => Pin::new(&mut stream).encrypt(),
                TlsMode::StartTls => { /* ready! */ }
            }
            Ok(stream)
        })
    }
}

struct ChildIo {
    inner: Option<Child>,
    closing: Option<Pin<Box<dyn Future<Output = io::Result<Output>> + Sync + Send>>>,
}

impl Read for ChildIo {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<io::Result<usize>> {
        if let Some(r) = self.inner.as_mut().and_then(|i| i.stdout.as_mut()) {
            Pin::new(r).poll_read(cx, buf)
        } else {
            Poll::Ready(Err(io::ErrorKind::BrokenPipe.into()))
        }
    }
}
impl Write for ChildIo {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<io::Result<usize>> {
        if let Some(w) = self.inner.as_mut().and_then(|i| i.stdin.as_mut()) {
            Pin::new(w).poll_write(cx, buf)
        } else {
            Poll::Ready(Err(io::ErrorKind::BrokenPipe.into()))
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        if let Some(w) = self.inner.as_mut().and_then(|i| i.stdin.as_mut()) {
            Pin::new(w).poll_flush(cx)
        } else {
            Poll::Ready(Err(io::ErrorKind::BrokenPipe.into()))
        }
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        if let Some(w) = self.inner.as_mut().and_then(|i| i.stdin.as_mut()) {
            ready!(Pin::new(w).poll_close(cx))?;

            if let Some(inner) = self.inner.take() {
                self.closing = Some(Box::pin(inner.output()));
            }
        }

        if let Some(ref mut closing) = self.closing {
            let output = ready!(closing.as_mut().poll(cx))?;
            self.closing = None;

            if !output.status.success() {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::Other,
                    ErrorMessage {
                        data: output.stderr,
                    },
                )));
            }
        }

        Poll::Ready(Ok(()))
    }
}

#[derive(Debug)]
struct ErrorMessage {
    data: Vec<u8>,
}
impl Display for ErrorMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Child process error:\n{}",
            String::from_utf8_lossy(self.data.as_slice())
        ))
    }
}
impl std::error::Error for ErrorMessage {}
