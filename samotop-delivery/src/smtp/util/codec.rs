use async_std::io::{self, Read, Write};
use async_std::prelude::*;
use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// The codec used for transparency
/// TODO: replace CR and LF by CRLF
/// TODO: check line length
/// FIXME: Fix transfer encoding based on available ESMTP extensions.
///        That means also to understand and update MIME headers.
#[derive(Default, Clone, Copy, Debug)]
#[cfg_attr(
    feature = "serde-impls",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub struct SmtpDataCodec {
    escape_count: u8,
}

impl SmtpDataCodec {
    /// Creates a new client codec
    pub fn new() -> Self {
        SmtpDataCodec::default()
    }
}

impl SmtpDataCodec {
    /// Close the data stream - this writes the appropriate final dot
    pub async fn close<W: Write + Unpin>(&mut self, buf: W) -> io::Result<()> {
        let mut buf = BugIO { inner: buf };
        match self.escape_count {
            0 => buf.write_all(b"\r\n.\r\n").await?,
            1 => buf.write_all(b"\n.\r\n").await?,
            2 => buf.write_all(b".\r\n").await?,
            _ => unreachable!(),
        }
        self.escape_count = 0;
        Ok(())
    }
    /// Encode data - it does not handle final dot
    pub async fn encode<W: Write + Unpin>(&mut self, frame: &[u8], buf: W) -> io::Result<()> {
        let mut buf = BugIO { inner: buf };
        let mut start = 0;
        for (idx, byte) in frame.iter().enumerate() {
            match self.escape_count {
                0 => self.escape_count = if *byte == b'\r' { 1 } else { 0 },
                1 => self.escape_count = if *byte == b'\n' { 2 } else { 0 },
                2 => self.escape_count = if *byte == b'.' { 3 } else { 0 },
                _ => unreachable!(),
            }
            if self.escape_count == 3 {
                self.escape_count = 0;
                buf.write_all(&frame[start..idx]).await?;
                buf.write_all(b".").await?;
                start = idx;
            }
        }
        buf.write_all(&frame[start..]).await?;
        Ok(())
    }
}

#[pin_project]
#[derive(Default, Debug, Clone)]
pub struct BugIO<S> {
    #[pin]
    pub inner: S,
}

impl<S: Read> Read for BugIO<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        let res = self.project().inner.poll_read(cx, buf);
        debug!("poll_read {:?} {:?}", res, std::str::from_utf8(buf));
        res
    }
}
impl<S: Write> Write for BugIO<S> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let res = self.project().inner.poll_write(cx, buf);
        debug!("poll_write {:?} {:?}", res, std::str::from_utf8(buf));
        res
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let res = self.project().inner.poll_flush(cx);
        debug!("poll_flush {:?}", res);
        res
    }
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let res = self.project().inner.poll_close(cx);
        debug!("poll_close");
        res
    }
}
