use super::delay;
use crate::common::*;
use std::time::Duration;

pub struct ReadTimeoutIo<IO> {
    expired: Option<Pin<Box<dyn Future<Output = ()> + Sync + Send>>>,
    timeout: Duration,
    io: Pin<Box<IO>>,
}

impl<IO> ReadTimeoutIo<IO> {
    pub fn new(timeout: Duration, io: IO) -> Self {
        ReadTimeoutIo {
            expired: None,
            timeout,
            io: Box::pin(io),
        }
    }
}

impl<IO: io::Read> io::Read for ReadTimeoutIo<IO> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        if let Some(ref mut expired) = self.expired {
            if expired.as_mut().poll(cx).is_ready() {
                // too much time passed since last read/write
                return Poll::Ready(Err(io::ErrorKind::TimedOut.into()));
            }
        }

        let res = self.io.as_mut().poll_read(cx, buf);

        match res {
            Poll::Pending => {
                if self.expired.is_none() {
                    // No data, start checking for a timeout
                    // Poll once to register waker!
                    self.expired =
                        delay(self.timeout).and_then(|mut fut| match fut.as_mut().poll(cx) {
                            Poll::Ready(_) => None,
                            Poll::Pending => Some(fut),
                        });
                }
            }
            Poll::Ready(_) => self.expired = None,
        }

        res
    }
}
impl<IO: io::Write> io::Write for ReadTimeoutIo<IO> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.io.as_mut().poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.io.as_mut().poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.io.as_mut().poll_close(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_std::io::ReadExt;
    use async_std::io::{copy, Cursor};
    use async_std::prelude::FutureExt;
    use std::time::Duration;

    #[async_std::test]
    async fn pass_read_with_no_timeout() {
        let input = Cursor::new(b"some".to_vec());
        let mut output = Cursor::new(b"".to_vec());
        let io = ReadTimeoutIo::new(Duration::ZERO, input);

        insta::assert_debug_snapshot!((copy(io,&mut output).await,output),@r###"
        (
            Ok(
                4,
            ),
            Cursor {
                inner: Cursor {
                    inner: [
                        115,
                        111,
                        109,
                        101,
                    ],
                    pos: 4,
                },
            },
        )
        "###);
    }

    #[async_std::test]
    async fn pass_read_within_timeout() {
        let input = Cursor::new(b"some".to_vec());
        let output = Cursor::new(b"".to_vec());
        let io = ReadTimeoutIo::new(Duration::from_micros(100), input);

        insta::assert_debug_snapshot!(copy(io,output).await,@r###"
        Ok(
            4,
        )
        "###);
    }

    #[async_std::test]
    async fn fail_read_after_timeout() -> io::Result<()> {
        let mut output = b"______".to_vec();
        let io = PendIo;
        let mut io = ReadTimeoutIo::new(Duration::from_millis(5), io);
        let mut io = Pin::new(&mut io);
        insta::assert_debug_snapshot!(io.read(&mut output[..]).timeout(Duration::from_secs(1)).await,@r###"
        Ok(
            Err(
                Kind(
                    TimedOut,
                ),
            ),
        )
        "###);
        Ok(())
    }

    #[async_std::test]
    async fn timeout_expires() {
        let later = delay(Duration::from_millis(1)).expect("some").await;
        insta::assert_debug_snapshot!(later,@r"()");
    }

    /// Mock IO always pending
    struct PendIo;
    impl io::Read for PendIo {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _buf: &mut [u8],
        ) -> Poll<futures_io::Result<usize>> {
            Poll::Pending
        }
    }
    impl io::Write for PendIo {
        fn poll_write(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _buf: &[u8],
        ) -> Poll<futures_io::Result<usize>> {
            Poll::Pending
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<futures_io::Result<()>> {
            Poll::Pending
        }

        fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<futures_io::Result<()>> {
            Poll::Pending
        }
    }
}
