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
    pub mod io {
        pub use futures_io::{
            AsyncBufRead as BufRead, AsyncRead as Read, AsyncSeek as Seek, AsyncWrite as Write,
        };
        pub use std::io::{Error, ErrorKind, Result};
    }
    //pub use async_std::io;
    //pub use async_std::io::prelude::{ReadExt, WriteExt};
    //pub use async_std::io::{Read, Write};
    pub use futures_core::ready;
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

    // pub async fn ready<T>(val: T) -> T {
    //     val
    // }

    // replace with std once stable - use of unstable library feature 'future_poll_fn'
    pub async fn poll_fn<F, T>(f: F) -> T
    where
        F: FnMut(&mut Context<'_>) -> Poll<T>,
    {
        let fut = PollFn { f };
        fut.await
    }

    struct PollFn<F> {
        f: F,
    }

    impl<F> Unpin for PollFn<F> {}

    impl<T, F> Future for PollFn<F>
    where
        F: FnMut(&mut Context<'_>) -> Poll<T>,
    {
        type Output = T;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
            (&mut self.f)(cx)
        }
    }

    /// In the absence of random number generator produces a time based identifier
    /// It is not reliable nor secure, RNG/PRNG should be preffered.
    pub fn time_based_id() -> String {
        fn nonnumber(input: char) -> bool {
            !input.is_ascii_digit()
        }
        // for the lack of better unique string without extra dependencies
        format!("{:?}", std::time::Instant::now()).replace(nonnumber, "")
    }
}
