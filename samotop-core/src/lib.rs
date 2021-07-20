//! The domain model of Samotop and core functionality. A base crate for samotop extensions.

#[macro_use]
extern crate log;

pub mod io;
pub mod mail;
pub mod smtp;

#[cfg(feature = "server")]
pub mod server;

pub mod common {
    pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
    pub type Result<T> = std::result::Result<T, Error>;
    pub use async_std::future::poll_fn;
    pub use async_std::future::ready;
    pub use async_std::io;
    pub use async_std::io::prelude::{ReadExt, WriteExt};
    pub use async_std::io::{Read, Write};
    pub use async_std::task::ready;
    pub use std::future::*;
    pub type S3Fut<T> = Pin<Box<dyn Future<Output = T> + Sync + Send + 'static>>;
    pub type S2Fut<'a, T> = Pin<Box<dyn Future<Output = T> + Sync + Send + 'a>>;
    pub type S1Fut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
    pub use std::fmt;
    pub use std::pin::Pin;
    pub use std::sync::Arc;
    pub use std::task::{Context, Poll};

    #[derive(Debug, Copy, Clone)]
    pub struct Dummy;

    /// In the absence of random number generator produces a time based identifier
    /// It is not reliable nor secure, RNG/PRNG should be preffered.
    pub fn time_based_id() -> String {
        fn nonnumber(input: char) -> bool {
            !input.is_ascii_digit()
        }
        // for the lack of better unique string without extra dependencies
        format!("{:?}", std::time::Instant::now()).replace(nonnumber, "")
    }

    /// Enable async close
    /// TODO: remove after https://github.com/async-rs/async-std/issues/977
    pub trait WriteClose {
        fn close(&mut self) -> CloseFuture<'_, Self>
        where
            Self: Unpin;
    }
    impl<T> WriteClose for T
    where
        T: Write,
    {
        /// Closes the writer.
        fn close(&mut self) -> CloseFuture<'_, Self>
        where
            Self: Unpin,
        {
            CloseFuture { writer: self }
        }
    }

    /// Future for the [`AsyncWriteExt::close()`] method.
    /// Async close future
    /// TODO: remove after https://github.com/async-rs/async-std/issues/977
    #[derive(Debug)]
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct CloseFuture<'a, W: Unpin + ?Sized> {
        writer: &'a mut W,
    }

    impl<W: Unpin + ?Sized> Unpin for CloseFuture<'_, W> {}

    impl<W: Write + Unpin + ?Sized> Future for CloseFuture<'_, W> {
        type Output = io::Result<()>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            Pin::new(&mut *self.writer).poll_close(cx)
        }
    }
}
