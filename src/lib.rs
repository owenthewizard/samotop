//! # Status
//!
//! The API is still very much subject to change. Until you see the release of version 1.0.0, don't expect much stability.
//! See the README.md file and project open issues for current status.
//!
//! The use case of running the server as a standalone application should be described in the README.md (tbd)
//! Here we focus on using the library.
//!
//! # Installation
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! samotop = "0"
//! ```
//!
//! Note that the API is still unstable. Please use the latest release.
//!
//! # Usage
//!
//! There are a few interesting provisions one could take away here:
//! * The server (through `samotop::server::Server`) - it takes IP:port's to listen `on()` and you can then `serve()` your own implementation of a `TcpService`.
//! * The SMTP service (`SmtpService`) - it takes a `async_std::io::net::TcpStream` into the `Sink` created by `start()`.
//! * The low level `SmtpCodec` - it implements `futures_codec::Encoder` and `futures_codec::Decoder`. It handles SMTP mail data as well.
//! * The SMTP session parser (`SmtpParser`) - it takes `&str` and returns parsed commands or session.
//! * The SMTP session and domain model (`model::session`, `model::command`, `model::response`) - these describe the domain and behavior.
//! * The mail handling stuff that is yet to be written (`MailService`)...
//!
//! The individual components may later be split out into their own crates, but we shall have the samotop crate re-export them then.
//!
//! # Server
//! Any TCP service can be served. See the docs for `TcpService`.
//!
//! ```no_run
//! extern crate async_std;
//! extern crate env_logger;
//! extern crate samotop;
//!
//! use samotop::server::Server;
//! use samotop::service::tcp::DummyTcpService;
//!
//! fn main() {
//!     env_logger::init();
//!     let mut srv = Server::on("localhost:0");
//!     async_std::task::block_on(srv.serve(DummyTcpService)).unwrap()
//! }
//! ```

#[macro_use]
extern crate log;

pub mod grammar;
pub mod model;
pub mod protocol;
pub mod server;
pub mod service;

mod common {
    pub use crate::model::{Error, Result};

    pub use futures::future::FutureExt;
    pub use futures::prelude::{future, Future, Sink, Stream};
    pub use futures::ready;
    pub use futures::stream::StreamExt;

    pub use async_std::io::prelude::{Read, ReadExt, Write, WriteExt};
    pub use bytes::{Bytes, BytesMut};
    pub use pin_project::pin_project;
    pub use std::pin::Pin;
    pub use std::sync::Arc;
    pub use std::task::{Context, Poll};
}

#[cfg(test)]
pub mod test_util {

    pub use crate::common::*;
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
        pub data: Vec<u8>,
        pub split: usize,
        pub read: usize,
        pub read_chunks: VecDeque<usize>,
    }
    impl TestIO {
        // Pretend reading chunks of input of given sizes. 0 => Pending
        pub fn read_chunks(&mut self, parts: impl IntoIterator<Item = usize>) {
            self.read_chunks = parts.into_iter().collect()
        }
        pub fn written(&self) -> &[u8] {
            &self.data[self.split..]
        }
        pub fn read(&self) -> &[u8] {
            &self.data[..self.read]
        }
    }
    impl<T: IntoIterator<Item = u8>> From<T> for TestIO {
        fn from(data: T) -> Self {
            let data: Vec<u8> = data.into_iter().collect();
            let len = data.len();
            TestIO {
                split: len,
                read: 0,
                data,
                read_chunks: vec![len].into(),
            }
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
                    let len = usize::min(max, *proj.split - *proj.read);
                    let len = usize::min(len, buf.len());
                    if len != max {
                        proj.read_chunks.push_front(max - len);
                    }
                    if len == 0 {
                        Poll::Pending
                    } else {
                        (&mut buf[..len]).copy_from_slice(&proj.data[*proj.read..*proj.read + len]);
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
            proj.data.push(buf[0]);
            Poll::Ready(Ok(1))
        }
        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }
        fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }
}
