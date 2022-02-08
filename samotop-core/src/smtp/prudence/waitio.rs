use super::delay;
use crate::common::*;
use futures_core::ready;
use std::time::Duration;

/// Delays writes and asserts that no reads come before a delay - one must wait!
pub struct WaitIo<IO> {
    writes: Vec<u8>,
    state: State,
    io: Pin<Box<IO>>,
}

impl<IO> WaitIo<IO> {
    pub fn new(wait: Duration, io: IO) -> Self {
        WaitIo {
            writes: vec![],
            state: match wait.is_zero() {
                true => State::Started(None),
                false => State::New(wait),
            },
            io: Box::pin(io),
        }
    }
}

impl<IO: io::Read> io::Read for WaitIo<IO> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        // registers the wake up
        self.state.waiting(cx);

        let res = Pin::new(&mut self.io).poll_read(cx, buf);

        if let Poll::Ready(Ok(len)) = res {
            if len > 0 && self.state.waiting(cx) {
                // We have data, but the delay did not pass yet.
                // That means the client sent data before seeing the banner.
                return Poll::Ready(Err(io::ErrorKind::ConnectionRefused.into()));
            }
        }

        res
    }
}
impl<IO: io::Write> io::Write for WaitIo<IO> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        if self.state.waiting(cx) {
            // still waiting, delay the writes
            self.writes.extend_from_slice(buf);
            return Poll::Ready(Ok(buf.len()));
        }

        if !self.writes.is_empty() {
            // flush delayed writes before writing current
            ready!(self.as_mut().poll_flush(cx))?;
        }

        self.io.as_mut().poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.state.waiting(cx) {
            return Poll::Pending;
        }

        while !self.writes.is_empty() {
            // flush delayed writes, it may go per partes
            let mut buf = std::mem::take(&mut self.writes);
            match self.as_mut().io.as_mut().poll_write(cx, &buf[..]) {
                Poll::Ready(Ok(len)) => {
                    self.writes = buf.split_off(len);
                }
                Poll::Ready(Err(e)) => {
                    self.writes = buf;
                    return Poll::Ready(Err(e));
                }
                Poll::Pending => {
                    self.writes = buf;
                    return Poll::Pending;
                }
            };
        }
        self.io.as_mut().poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // Unblock so we can flush
        self.state = State::default();
        ready!(self.as_mut().poll_flush(cx))?;
        self.io.as_mut().poll_close(cx)
    }
}

enum State {
    New(Duration),
    Started(Option<Pin<Box<dyn Future<Output = ()> + Sync + Send>>>),
}
impl Default for State {
    fn default() -> Self {
        State::Started(None)
    }
}
impl State {
    fn started(&mut self) -> Option<&mut Pin<Box<dyn Future<Output = ()> + Sync + Send>>> {
        if let State::New(wait) = self {
            // first, create delay
            *self = State::Started(delay(*wait));
        }

        if let State::Started(ref mut wait) = self {
            wait.as_mut()
        } else {
            None
        }
    }
    fn waiting(&mut self, cx: &mut Context<'_>) -> bool {
        let waiting = self
            .started()
            .map(|wait| wait.as_mut().poll(cx).is_pending())
            .unwrap_or_default();
        if !waiting {
            *self = State::default()
        }
        waiting
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_std::{
        io::{copy, Cursor},
        task::sleep,
    };
    use std::time::{Duration, Instant};

    #[async_std::test]
    async fn pass_read_with_no_delay() {
        let input = Cursor::new(b"some".to_vec());
        let mut output = Cursor::new(b"".to_vec());
        let io = WaitIo::new(Duration::ZERO, input);

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
    async fn fail_read_before_delay() {
        let input = Cursor::new(b"some".to_vec());
        let output = Cursor::new(b"".to_vec());
        let io = WaitIo::new(Duration::from_secs(10), input);
        insta::assert_debug_snapshot!(copy(io,output).await,@r###"
        Err(
            Custom {
                kind: ConnectionRefused,
                error: VerboseError {
                    source: Kind(
                        ConnectionRefused,
                    ),
                    message: "io::copy failed",
                },
            },
        )
        "###);
    }

    #[async_std::test]
    async fn pass_read_after_delay() {
        let input = Cursor::new(b"some".to_vec());
        let output = Cursor::new(b"".to_vec());
        let io = WaitIo::new(Duration::from_micros(10), input);
        let _ = sleep(Duration::from_millis(100)).await;
        insta::assert_debug_snapshot!(copy(io,output).await,@r###"
        Ok(
            4,
        )
        "###);
    }

    #[async_std::test]
    async fn delay_writes_before_delay() {
        let input = Cursor::new(b"some".to_vec());
        let mut output = Cursor::new(b"".to_vec());
        let io = WaitIo::new(Duration::from_millis(10), &mut output);
        let start = Instant::now();
        insta::assert_debug_snapshot!((copy(input, io).await,output),@r###"
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
        assert!(start.elapsed().as_millis() >= 10);
    }

    #[async_std::test]
    async fn pass_writes_with_no_delay() {
        let input = Cursor::new(b"some".to_vec());
        let mut output = Cursor::new(b"f".to_vec());
        let io = WaitIo::new(Duration::ZERO, &mut output);
        let start = Instant::now();
        insta::assert_debug_snapshot!((copy(input, io).await,output),@r###"
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
        assert!(start.elapsed().as_millis() < 1);
    }
}
