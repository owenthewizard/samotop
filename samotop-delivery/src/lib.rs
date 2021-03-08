//! samotop-delivery is a set of transports to deliver mail to,
//! notably to SMTP/LMTP, but also maildir... It is used in Samotop
//! as a delivery solution for incoming mail.
//!
//! ## Example
//!
//! ```rust
//! pub type Error = Box<dyn std::error::Error + Send + Sync>;
//! pub type Result<T> = std::result::Result<T, Error>;
//!
//! use samotop_delivery::prelude::{
//!     Envelope, SmtpClient, Transport,
//! };
//!
//! async fn smtp_transport_simple() -> Result<()> {
//!     let envelope = Envelope::new(
//!             Some("user@localhost".parse()?),
//!             vec!["root@localhost".parse()?],
//!             "id".to_string(),
//!         )?;
//!     let message = "From: user@localhost\r\n\
//!                     Content-Type: text/plain\r\n\
//!                     \r\n\
//!                     Hello example"
//!                     .as_bytes();
//!     let client = SmtpClient::new("127.0.0.1:2525")?;
//!     
//!     // Create a client, connect and send
//!     client.connect_and_send(envelope, message).await?;    
//!
//!     Ok(())
//! }
//! ```

#![deny(
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    missing_debug_implementations,
    clippy::unwrap_used
)]

#[macro_use]
extern crate log;

#[cfg(feature = "dir-transport")]
pub mod dir;
mod dispatch;
#[cfg(feature = "file-transport")]
pub mod file;
#[cfg(feature = "journal-transport")]
pub mod journal;
#[cfg(feature = "sendmail-transport")]
pub mod sendmail;
#[cfg(feature = "smtp-transport")]
pub mod smtp;
pub mod stub;
pub mod types;

pub mod prelude {
    #[cfg(feature = "dir-transport")]
    pub use crate::dir::*;
    #[cfg(feature = "file-transport")]
    pub use crate::file::*;
    #[cfg(feature = "journal-transport")]
    pub use crate::journal::*;
    #[cfg(feature = "sendmail-transport")]
    pub use crate::sendmail::*;
    #[cfg(feature = "smtp-transport")]
    pub use crate::smtp::*;
    pub use crate::types::*;
    pub use crate::{MailDataStream, Transport};
}

use crate::types::*;
use async_std::io::{copy, Read, Write};
use futures::{io::AsyncWriteExt, Future};
use std::{fmt, pin::Pin};

pub type SyncFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Sync + Send + 'a>>;
type SendResult<T> = Result<<T as MailDataStream>::Output, <T as MailDataStream>::Error>;

/// Transport method for emails
pub trait Transport: std::fmt::Debug {
    /// Result type for the transport
    type DataStream: MailDataStream;

    /// Start sending e-mail and return a stream to write the body to
    fn send_stream<'s, 'a>(
        &'s self,
        envelope: Envelope,
    ) -> SyncFuture<'a, Result<Self::DataStream, <Self::DataStream as MailDataStream>::Error>>
    where
        's: 'a;

    /// Send the email
    fn send<'s, 'r, 'a, R>(
        &'s self,
        envelope: Envelope,
        message: R,
    ) -> SyncFuture<'a, SendResult<Self::DataStream>>
    where
        Self::DataStream: Unpin + Send + Sync,
        <Self::DataStream as MailDataStream>::Error: From<std::io::Error>,
        R: Read + Unpin + Send + Sync + 'r,
        's: 'a,
        'r: 'a,
    {
        let stream = self.send_stream(envelope);
        Box::pin(async move {
            let mut stream = stream.await?;
            copy(message, &mut stream).await?;
            stream.close().await?;
            stream.result()
        })
    }
}

pub trait MailDataStream: fmt::Debug + Write {
    type Output;
    type Error;
    /// Return the result of sending the mail.
    /// This should return error if the mail has not been fully dispatched.
    /// In other words, it should fail if the mail data stream hasn't been closed.
    fn result(&self) -> Result<Self::Output, Self::Error>;
}
