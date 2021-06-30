use crate::MailDataStream;
use async_std::fs::File;
use futures::{ready, AsyncWrite as Write, Future};
use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

#[pin_project(project=MailFileProj)]
pub struct MailFile {
    id: String,
    file: File,
    target: Pin<Box<dyn Future<Output = std::io::Result<()>> + Send + Sync + 'static>>,
    closed: bool,
}

impl MailFile {
    pub fn new(
        id: String,
        file: File,
        target: Pin<Box<dyn Future<Output = std::io::Result<()>> + Send + Sync + 'static>>,
    ) -> Self {
        Self {
            id,
            file,
            target,
            closed: false,
        }
    }
}
impl std::fmt::Debug for MailFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MailFile")
            .field("id", &self.id)
            .field("file", &self.file)
            .finish()
    }
}
impl Write for MailFile {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        debug!(
            "poll_write: Writing data for {}: {} bytes.",
            self.id,
            buf.len()
        );
        Pin::new(self.project().file).poll_write(cx, buf)
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        trace!("poll_flush: Flushing data for {}.", self.id);
        Pin::new(self.project().file).poll_flush(cx)
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        trace!("poll_close");
        ready!(self.as_mut().poll_flush(cx))?;
        let MailFileProj {
            target,
            file,
            closed,
            ..
        } = self.project();
        trace!("closing file");
        ready!(Pin::new(file).poll_close(cx))?;
        trace!("moving file");
        ready!(target.as_mut().poll(cx))?;
        trace!("DONE!");
        *closed = true;
        Poll::Ready(Ok(()))
    }
}

impl MailDataStream for MailFile {
    fn is_done(&self) -> bool {
        self.closed
    }
}
