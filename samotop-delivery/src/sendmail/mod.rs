//! The sendmail transport sends the envelope using the local sendmail command.
//!

mod error;
pub use self::error::*;
use crate::{sendmail::error::Error, SyncFuture};
use crate::{Envelope, MailDataStream, Transport};
use async_std::io::Write;
use async_std::task;
use futures::{ready, Future};
use std::ops::DerefMut;
use std::pin::Pin;
use std::process::{Child, Command, Stdio};
use std::task::{Context, Poll};
use std::{convert::AsRef, fmt};

/// Sends an envelope using the `sendmail` command
#[derive(Debug, Default)]
#[cfg_attr(
    feature = "serde-impls",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub struct SendmailTransport {
    command: String,
}

impl SendmailTransport {
    /// Creates a new transport with the default `/usr/sbin/sendmail` command
    pub fn new() -> SendmailTransport {
        SendmailTransport {
            command: "/usr/sbin/sendmail".to_string(),
        }
    }

    /// Creates a new transport to the given sendmail command
    pub fn new_with_command<S: Into<String>>(command: S) -> SendmailTransport {
        SendmailTransport {
            command: command.into(),
        }
    }
}

impl Transport for SendmailTransport {
    type DataStream = ProcStream;
    type Error = Error;
    fn send_stream<'s, 'a>(&'s self, envelope: Envelope) -> SyncFuture<Result<ProcStream, Error>>
    where
        's: 'a,
    {
        let command = self.command.clone();
        let message_id = envelope.message_id().to_string();

        let from = envelope
            .from()
            .map(AsRef::as_ref)
            .unwrap_or("\"\"")
            .to_owned();
        let to = envelope.to().to_owned();

        Box::pin(async move {
            Ok(ProcStream::Ready(ProcStreamInner {
                child: Command::new(command)
                    .arg("-i")
                    .arg("-f")
                    .arg(from)
                    .args(to)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()
                    .map_err(Error::Io)?,
                message_id,
            }))
        })
    }
}

#[allow(missing_debug_implementations)]
pub enum ProcStream {
    Busy,
    Ready(ProcStreamInner),
    Closing(Pin<Box<dyn Future<Output = std::io::Result<()>> + Send + Sync>>),
    Done,
}

#[derive(Debug)]
pub struct ProcStreamInner {
    child: Child,
    message_id: String,
}

impl MailDataStream for ProcStream {
    fn is_done(&self) -> bool {
        match self {
            ProcStream::Done => true,
            _ => false,
        }
    }
}

/// Todo: async when available
impl Write for ProcStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        loop {
            break match self.deref_mut() {
                ProcStream::Ready(ref mut inner) => {
                    use std::io::Write;
                    let len = inner.child.stdin.as_mut().ok_or_else(broken)?.write(buf)?;
                    Poll::Ready(Ok(len))
                }
                mut otherwise => {
                    ready!(Pin::new(&mut otherwise).poll_flush(cx))?;
                    continue;
                }
            };
        }
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        loop {
            break match self.deref_mut() {
                ProcStream::Ready(ref mut inner) => {
                    use std::io::Write;
                    inner.child.stdin.as_mut().ok_or_else(broken)?.flush()?;
                    Poll::Ready(Ok(()))
                }
                ProcStream::Closing(ref mut fut) => {
                    ready!(fut.as_mut().poll(cx))?;
                    *self = ProcStream::Done;
                    continue;
                }
                ProcStream::Done => Poll::Ready(Ok(())),
                ProcStream::Busy => Poll::Ready(Err(broken())),
            };
        }
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        loop {
            break match std::mem::replace(self.deref_mut(), ProcStream::Busy) {
                ProcStream::Ready(ProcStreamInner { child, message_id }) => {
                    let fut = async move {
                        let output = task::spawn_blocking(move || child.wait_with_output()).await?;

                        info!("Wrote {} message to stdin", message_id);

                        if output.status.success() {
                            Ok(())
                        } else {
                            Err(std::io::Error::new(
                                std::io::ErrorKind::Other,
                                String::from_utf8_lossy(output.stderr.as_slice()),
                            ))
                        }
                    };
                    *self = ProcStream::Closing(Box::pin(fut));
                    continue;
                }
                otherwise @ ProcStream::Closing(_) => {
                    *self = otherwise;
                    ready!(Pin::new(&mut self).poll_flush(cx))?;
                    continue;
                }
                otherwise => {
                    *self = otherwise;
                    ready!(Pin::new(&mut self).poll_flush(cx))?;
                    Poll::Ready(Ok(()))
                }
            };
        }
    }
}

fn broken() -> std::io::Error {
    std::io::Error::from(std::io::ErrorKind::NotConnected)
}

impl fmt::Debug for ProcStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcStream::Busy => f.debug_tuple("Busy").finish(),
            ProcStream::Done => f.debug_tuple("Done").finish(),
            ProcStream::Closing(_) => f.debug_tuple("Closing").field(&"*").finish(),
            ProcStream::Ready(ref r) => f.debug_tuple("Ready").field(&r).finish(),
        }
    }
}
