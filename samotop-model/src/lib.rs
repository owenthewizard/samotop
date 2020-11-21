#[macro_use]
extern crate log;

pub mod io;
pub mod mail;
pub mod parser;
pub mod smtp;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

pub mod common {
    pub use crate::{Error, Result};
    pub use futures_io::AsyncBufRead as BufRead;
    pub use futures_io::AsyncRead as Read;
    pub use futures_io::AsyncWrite as Write;
    pub use std::future::*;
    pub type S3Fut<T> = Pin<Box<dyn Future<Output = T> + Sync + Send + 'static>>;
    pub type S2Fut<'a, T> = Pin<Box<dyn Future<Output = T> + Sync + Send + 'a>>;
    pub type SendFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
    // pub use futures::{
    //     future, future::BoxFuture, ready, stream, AsyncRead as Read, AsyncReadExt as ReadExt,
    //     AsyncWrite as Write, AsyncWriteExt as WriteExt, Future, FutureExt, Stream, StreamExt,
    //     TryFutureExt,
    // };
    //pub use pin_project::pin_project;
    pub use std::pin::Pin;
    pub use std::sync::Arc;
    pub use std::task::{Context, Poll};

    //TODO: remove when ready() is stabilised in std
    pub async fn ready<T>(item: T) -> T {
        item
    }

    /// TODO: Remove when poll_fn() is stabilized in std
    pub fn poll_fn<T, F>(f: F) -> PollFn<F>
    where
        F: FnMut(&mut Context<'_>) -> Poll<T>,
    {
        PollFn { f }
    }

    /// A Future that wraps a function returning `Poll`.
    ///
    /// This `struct` is created by [`poll_fn()`]. See its
    /// documentation for more.
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct PollFn<F> {
        f: F,
    }

    impl<F> Unpin for PollFn<F> {}

    impl<F> std::fmt::Debug for PollFn<F> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("PollFn").finish()
        }
    }

    impl<T, F> Future for PollFn<F>
    where
        F: FnMut(&mut Context<'_>) -> Poll<T>,
    {
        type Output = T;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
            (&mut self.f)(cx)
        }
    }
}
