//! # Status
//!
//! Reaching stable. The API builds on async/await to offer a convenient asynchronous interface.
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
//! There are a few interesting provisions one could take away from Samotop:
//! * The server (through `samotop::server::Server`) - it takes IP:port's to listen `on()` and you can then `serve()` your own implementation of a `TcpService`.
//! * The SMTP service (`SmtpService`) - it takes an async IO and provides an SMTP service defined by `SessionService`.
//! * The low level `SmtpCodec` - it translates between IO and a `Stram` of `ReadControl` and a `Sink` of `WriteControl`. It handles SMTP mail data as well.
//! * The SMTP session parser (`SmtpParser`) - it takes `&str` and returns parsed commands or session.
//! * The SMTP session and domain model (`samotop::model::session`, `samotop::model::smtp`) - these describe the domain and behavior.
//! * The mail handling stuff that is yet to be written (`MailService`)...
//!
//! # SMTP Server
//!
//! You can run a plaintext SMTP service without support for STARTTLS.
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
//!     let mail = samotop::service::mail::ConsoleMail::new("Aloha");
//!     let sess = samotop::service::session::StatefulSessionService::new(mail);
//!     let svc = samotop::service::tcp::SmtpService::new(sess);
//!     let svc = samotop::service::tcp::TlsEnabled::no(svc); //TLS disabled
//!     let srv = samotop::server::Server::on("localhost:25").serve(svc);
//!     async_std::task::block_on(srv).unwrap()
//! }
//! ```
//! 
//! To enable TLS, provide a rustls TlsAcceptor. 
//! Alternatively, implement TlsEnabled for another TLS library.
//!
//! # Dummy server
//! Any TCP service can be served. See the docs for `TcpService`.
//! Use this to understand how networking IO is handled.
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
//!     let mut srv = Server::on("localhost:0").serve(DummyTcpService);
//!     async_std::task::block_on(srv).unwrap()
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
    use crate::protocol::TlsCapableIO;
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
    impl TlsCapableIO for TestIO {
        fn start_tls(self: Pin<&mut Self>) -> Result<()> {
            Err("TLS not supported".into())
        }
    }
}
