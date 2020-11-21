//! samotop-delivery is an implementation of the smtp protocol client in Rust.
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
//!             Some("user@localhost".parse().unwrap()),
//!             vec!["root@localhost".parse().unwrap()],
//!             "id".to_string(),
//!         ).unwrap();
//!     let message = "From: user@localhost\r\n\
//!                     Content-Type: text/plain\r\n\
//!                     \r\n\
//!                     Hello example"
//!                     .as_bytes();
//!     let client = SmtpClient::new("127.0.0.1:2525").unwrap();
//!     
//!     // Create a client, connect and send
//!     client.connect_and_send(envelope, message).await.unwrap();    
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

pub mod delivery;
pub mod dir;
#[cfg(feature = "file-transport")]
pub mod file;
#[cfg(feature = "sendmail-transport")]
pub mod sendmail;
#[cfg(feature = "smtp-transport")]
pub mod smtp;
pub mod stub;
pub mod types;

pub mod prelude {
    #[cfg(feature = "file-transport")]
    pub use crate::file::FileTransport;
    #[cfg(feature = "sendmail-transport")]
    pub use crate::sendmail::SendmailTransport;
    #[cfg(feature = "smtp-transport")]
    pub use crate::smtp::{ClientSecurity, SmtpClient, SmtpTransport};
    pub use crate::types::*;
    pub use crate::{MailDataStream, Transport};
}

use crate::prelude::*;
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
