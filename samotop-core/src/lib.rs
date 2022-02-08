//! The domain model of Samotop and core functionality. A base crate for samotop extensions.

#[macro_use]
extern crate tracing;

pub mod config;
pub mod io;
pub mod mail;
pub mod smtp;

pub mod common {
    pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
    pub type Result<T> = std::result::Result<T, Error>;
    pub mod io {
        pub use futures_io::{
            AsyncBufRead as BufRead, AsyncRead as Read, AsyncSeek as Seek, AsyncWrite as Write,
        };
        pub use std::io::{Error, ErrorKind, Result};
    }
    pub use futures_core::ready;
    pub use futures_core::Stream;
    use std::any::TypeId;
    pub use std::future::*;
    pub type S3Fut<T> = Pin<Box<dyn Future<Output = T> + Sync + Send + 'static>>;
    pub type S2Fut<'a, T> = Pin<Box<dyn Future<Output = T> + Sync + Send + 'a>>;
    pub type S1Fut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
    pub use std::fmt;
    pub use std::pin::Pin;
    use std::sync::atomic::AtomicU32;
    use std::sync::atomic::Ordering;
    pub use std::sync::Arc;
    pub use std::task::{Context, Poll};

    #[derive(Debug, Copy, Clone)]
    pub struct FallBack;

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
    #[deprecated(
        since = "0.13.1",
        note = "Use Identify::now() instead. This shall be removed in 0.14.0"
    )]
    pub fn time_based_id() -> String {
        Identify::now().to_string()
    }

    /// Provide identifying IDs based on run time info
    pub struct Identify;
    impl Identify {
        /// Establish and get static app ID
        pub fn instance() -> u32 {
            static INSTANCE: AtomicU32 = AtomicU32::new(0);
            // CHECKME: what about all these Ordering styles?
            let mut value = INSTANCE.load(Ordering::Relaxed);
            if value == 0 {
                value = Self::now();
                match INSTANCE.compare_exchange(0, value, Ordering::Relaxed, Ordering::Relaxed) {
                    Ok(_) => value,
                    Err(value) => value,
                }
            } else {
                value
            }
        }
        /// Get a current unique ID
        pub fn now() -> u32 {
            // for the lack of better unique string without extra dependencies
            Self::hash(
                format!(
                    "{}.{}.{:?}.{:?}",
                    env!("CARGO_PKG_VERSION"),
                    std::process::id(),
                    std::time::Instant::now(),
                    TypeId::of::<crate::smtp::SmtpSession>()
                )
                .as_str(),
            )
        }
        const fn hash(s: &str) -> u32 {
            let s = s.as_bytes();
            let mut hash = 3581u32;
            let mut i = 0usize;
            while i < s.len() {
                hash = hash.wrapping_mul(33).wrapping_add(s[i] as u32);
                i += 1;
            }
            hash
        }
    }
}

#[test]
fn identify_instance_is_not_zero() {
    assert_ne!(common::Identify::instance(), 0)
}
#[test]
fn identify_instance_stays_constant() {
    assert_eq!(common::Identify::instance(), common::Identify::instance())
}
#[test]
fn identify_now_is_not_zero() {
    assert_ne!(common::Identify::now(), 0)
}
#[test]
fn identify_now_is_unique() {
    assert_ne!(common::Identify::now(), common::Identify::now())
}
