use async_std::io::{self, Read, Write};
use async_std::prelude::*;
use memchr::memchr2;
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
    state: State,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde-impls",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
enum State {
    AfterCRLF,
    AfterCR,
    Midway,
}
impl Default for State {
    fn default() -> Self {
        State::AfterCRLF
    }
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
        trace!("close");
        let mut buf = BugIO { inner: buf };
        if State::AfterCRLF == self.state {
            buf.write_all(b".\r\n").await?;
        } else {
            buf.write_all(b"\r\n.\r\n").await?;
        }
        self.state = State::default();
        Ok(())
    }
    /// Encode data - it does not handle final dot
    pub async fn encode<W: Write + Unpin>(&mut self, mut frame: &[u8], buf: W) -> io::Result<()> {
        debug!("encode {:?}", std::str::from_utf8(frame));
        let mut buf = BugIO { inner: buf };

        while !frame.is_empty() {
            // write an escape a dot after CR LF if the first char is a dot
            if State::AfterCRLF == self.state {
                if let Some(b'.') = frame.first() {
                    buf.write_all(b".".as_ref()).await?;
                }
            }
            // write the rest and manage state
            if let Some(pos) = memchr2(b'\n', b'\r', frame) {
                self.state = match frame[pos] {
                    // watch out, \n may follow
                    b'\r' => State::AfterCR,
                    // \n must immediately follow \r, otherwise it is not significant
                    b'\n' if pos == 0 && self.state == State::AfterCR => State::AfterCRLF,
                    // lone \n without \r
                    b'\n' => State::Midway,
                    _ => unreachable!(),
                };
                buf.write_all(&frame[..pos + 1]).await?;
                frame = &frame[pos + 1..];
            } else {
                self.state = State::Midway;
                buf.write_all(frame).await?;
                frame = &b""[..];
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    #[async_attributes::test]
    async fn test_dots_at_once() -> io::Result<()> {
        let mut buf: Vec<u8> = vec![];
        let mut sut = SmtpDataCodec::new();
        sut.encode(b"a\r\n.b\r\n..\r\n.\r\n", &mut buf).await?;
        assert_eq!(buf, b"a\r\n..b\r\n...\r\n..\r\n");
        Ok(())
    }
    #[async_attributes::test]
    async fn test_dots_per_partes() -> io::Result<()> {
        let mut buf: Vec<u8> = vec![];
        let mut sut = SmtpDataCodec::new();
        sut.encode(b"a\r\n", &mut buf).await?;
        sut.encode(b".", &mut buf).await?;
        sut.encode(b".\r\n", &mut buf).await?;
        sut.encode(b".", &mut buf).await?;
        sut.encode(b"\r\n", &mut buf).await?;
        assert_eq!(buf, b"a\r\n...\r\n..\r\n");
        Ok(())
    }
    #[async_attributes::test]
    async fn test_dots_colision() -> io::Result<()> {
        let mut buf: Vec<u8> = vec![];
        let mut sut = SmtpDataCodec::new();
        sut.encode(b"kwack\r\n", &mut buf).await?;
        sut.encode(b"\r\n", &mut buf).await?;
        sut.encode(b".\r\n", &mut buf).await?;
        assert_eq!(buf, b"kwack\r\n\r\n..\r\n");
        Ok(())
    }
    #[async_attributes::test]
    async fn test_dots_kwack2() -> io::Result<()> {
        let mut buf: Vec<u8> = vec![];
        let mut sut = SmtpDataCodec::new();
        sut.encode(b".kwack\r\n", &mut buf).await?;
        assert_eq!(buf, b"..kwack\r\n");
        Ok(())
    }
    #[async_attributes::test]
    async fn test_dots_boo() -> io::Result<()> {
        let mut buf: Vec<u8> = vec![];
        let mut sut = SmtpDataCodec::new();
        sut.encode(b"boo\r\n", &mut buf).await?;
        sut.encode(b".kwack\r\n", &mut buf).await?;
        assert_eq!(buf, b"boo\r\n..kwack\r\n");
        Ok(())
    }
    #[async_attributes::test]
    async fn test_dots_buggy_linefeed() -> io::Result<()> {
        let mut buf: Vec<u8> = vec![];
        let mut sut = SmtpDataCodec::new();
        sut.encode(b"boo\n", &mut buf).await?;
        sut.encode(b".gy", &mut buf).await?;
        assert_eq!(buf, b"boo\n.gy");
        Ok(())
    }
    #[async_attributes::test]
    async fn test_dots_buggy_cr() -> io::Result<()> {
        let mut buf: Vec<u8> = vec![];
        let mut sut = SmtpDataCodec::new();
        sut.encode(b"boo\r", &mut buf).await?;
        sut.encode(b".gy", &mut buf).await?;
        assert_eq!(buf, b"boo\r.gy");
        Ok(())
    }
    #[async_attributes::test]
    async fn test_dots_buggy_newline() -> io::Result<()> {
        let mut buf: Vec<u8> = vec![];
        let mut sut = SmtpDataCodec::new();
        sut.encode(b"\r.\n.", &mut buf).await?;
        assert_eq!(buf, b"\r.\n.");
        Ok(())
    }
    #[async_attributes::test]
    async fn test_dots_buggy_at_once() -> io::Result<()> {
        let mut buf: Vec<u8> = vec![];
        let mut sut = SmtpDataCodec::new();
        sut.encode(b"boo\r.gy\n..\n\n\n..", &mut buf).await?;
        assert_eq!(buf, b"boo\r.gy\n..\n\n\n..");
        Ok(())
    }
}
