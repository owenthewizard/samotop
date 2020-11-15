pub mod io;
pub mod mail;
pub mod smtp;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

pub mod common {
    pub use crate::{Error, Result};
    pub use std::future::*;
    pub type S3Fut<T> = Pin<Box<dyn Future<Output = T> + Sync + Send + 'static>>;
    pub type S2Fut<T> = Pin<Box<dyn Future<Output = T> + Sync + Send>>;
    // pub use futures::{
    //     future, future::BoxFuture, ready, stream, AsyncRead as Read, AsyncReadExt as ReadExt,
    //     AsyncWrite as Write, AsyncWriteExt as WriteExt, Future, FutureExt, Stream, StreamExt,
    //     TryFutureExt,
    // };
    //pub use pin_project::pin_project;
    pub use std::pin::Pin;
    pub use std::sync::Arc;
    pub use std::task::{Context, Poll};
}
