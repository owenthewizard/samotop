#[macro_use]
extern crate log;

pub mod protocol;
pub mod service;

pub mod model {
    pub use samotop_model::*;
}

pub mod common {
    pub use crate::model::io::Bytes;
    pub use crate::model::{Error, Result};

    pub use futures::{
        future, ready, stream, AsyncRead as Read, AsyncReadExt as ReadExt, AsyncWrite as Write,
        AsyncWriteExt as WriteExt, Future, FutureExt, Sink, Stream, StreamExt, TryFutureExt,
    };
    pub use pin_project::pin_project;
    pub use std::pin::Pin;
    pub use std::sync::Arc;
    pub use std::task::{Context, Poll};
}

pub mod test_util {

    pub use crate::common::*;
    use crate::protocol::tls::MayBeTls;
    use std::collections::VecDeque;

    pub fn cx() -> Context<'static> {
        std::task::Context::from_waker(futures::task::noop_waker_ref())
    }

    pub fn b(bytes: impl AsRef<[u8]>) -> Bytes {
        Bytes::copy_from_slice(bytes.as_ref())
    }

    #[pin_project]
    pub struct TestStream<I> {
        items: VecDeque<Poll<Option<I>>>,
    }
    impl<T: IntoIterator<Item = Poll<Option<I>>>, I> From<T> for TestStream<I> {
        fn from(from: T) -> Self {
            TestStream {
                items: from.into_iter().collect(),
            }
        }
    }
    impl<I> Stream for TestStream<I> {
        type Item = I;
        fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            if let Some(item) = self.project().items.pop_front() {
                item
            } else {
                Poll::Ready(None)
            }
        }
    }

    #[pin_project]
    pub struct TestIO {
        pub input: Vec<u8>,
        pub output: Vec<u8>,
        pub read: usize,
        pub read_chunks: VecDeque<usize>,
    }
    impl TestIO {
        pub fn written(&self) -> &[u8] {
            &self.output[..]
        }
        pub fn read(&self) -> &[u8] {
            &self.input[..self.read]
        }
        pub fn unread(&self) -> &[u8] {
            &self.input[self.read..]
        }
        pub fn new() -> Self {
            TestIO {
                output: vec![],
                input: vec![],
                read: 0,
                read_chunks: vec![].into(),
            }
        }
        // Pretend reading chunks of input of given sizes. 0 => Pending
        pub fn add_read_chunk(mut self, chunk: impl AsRef<[u8]>) -> Self {
            self.input.extend_from_slice(chunk.as_ref());
            self.read_chunks.push_back(chunk.as_ref().len());
            self
        }
    }
    impl<T: AsRef<[u8]>> From<T> for TestIO {
        fn from(data: T) -> Self {
            Self::new().add_read_chunk(data)
        }
    }
    impl Read for TestIO {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<std::io::Result<usize>> {
            let proj = self.project();
            match proj.read_chunks.pop_front() {
                None => Poll::Ready(Ok(0)),
                Some(max) => {
                    let len = usize::min(max, proj.input.len() - *proj.read);
                    let len = usize::min(len, buf.len());
                    if len != max {
                        proj.read_chunks.push_front(max - len);
                    }
                    if len == 0 {
                        Poll::Pending
                    } else {
                        (&mut buf[..len])
                            .copy_from_slice(&proj.input[*proj.read..*proj.read + len]);
                        *proj.read += len;
                        Poll::Ready(Ok(len))
                    }
                }
            }
        }
    }
    impl Write for TestIO {
        fn poll_write(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            let proj = self.project();
            proj.output.extend_from_slice(buf);
            Poll::Ready(Ok(buf.len()))
        }
        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }
        fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }
    impl MayBeTls for TestIO {
        fn start_tls(self: Pin<&mut Self>) -> std::io::Result<()> {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "TLS not supported",
            ))
        }
        fn supports_tls(&self) -> bool {
            false
        }
    }
}