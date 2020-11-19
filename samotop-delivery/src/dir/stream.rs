use crate::MailDataStream;
use async_std::fs::File;
use bytes::BytesMut;
use futures::{ready, AsyncWrite as Write, Future};
use log::*;
use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

use super::Error;

#[pin_project(project=MailFileProj)]
pub struct MailFile {
    id: String,
    file: File,
    buffer: BytesMut,
    target: Pin<Box<dyn Future<Output = std::io::Result<()>> + Send + Sync + 'static>>,
}

impl MailFile {
    pub fn new(
        id: String,
        file: File,
        buffer: BytesMut,
        target: Pin<Box<dyn Future<Output = std::io::Result<()>> + Send + Sync + 'static>>,
    ) -> Self {
        Self {
            id,
            file,
            buffer,
            target,
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
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        debug!("Mail data for {}: {} bytes", self.id, buf.len());
        if self.as_mut().buffer.len() > 10 * 1024 {
            ready!(self.as_mut().poll_flush(cx)?);
        }
        self.buffer.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let MailFileProj { file, buffer, .. } = self.project();
        let mut pending = &buffer[..];
        let mut file = Pin::new(file);
        trace!("Writing mail data: {} bytes", pending.len());
        while let Poll::Ready(len) = file.as_mut().poll_write(cx, pending)? {
            trace!("Wrote mail data: {} bytes", len);
            if len == 0 {
                break;
            }
            pending = &pending[len..];
        }
        // remove written bytes from the buffer
        let written = buffer.len() - pending.len();
        drop(buffer.split_to(written));
        trace!("Remaining {} bytes", buffer.len());
        if buffer.is_empty() {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        ready!(self.as_mut().poll_flush(cx))?;
        let MailFileProj { target, .. } = self.project();
        ready!(target.as_mut().poll(cx))?;
        Poll::Ready(Ok(()))
    }
}

impl MailDataStream for MailFile {
    type Output = ();

    type Error = Error;

    fn result(&self) -> Result<Self::Output, Self::Error> {
        todo!()
    }
}
