//! samotop-delivery is an implementation of the smtp protocol client in Rust.
//! ## Example
//!
//! ```rust
//! pub type Error = Box<dyn std::error::Error + Send + Sync>;
//! pub type Result<T> = std::result::Result<T, Error>;
//!
//! use samotop_delivery::{
//!     ClientSecurity, Envelope, SendableEmail, SmtpClient, Transport,
//! };
//!
//! async fn smtp_transport_simple() -> Result<()> {
//!     let email = SendableEmail::new(
//!         Envelope::new(
//!             Some("user@localhost".parse().unwrap()),
//!             vec!["root@localhost".parse().unwrap()],
//!         )?,
//!         "id",
//!         "Hello world",
//!     );
//!
//!     // Create a client, connect and send
//!     let _response = SmtpClient::new("127.0.0.1:2525")?.connect_and_send(email).await?;
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
use futures::io::AsyncWriteExt;
use samotop_async_trait::async_trait;

/// Transport method for emails
#[async_trait]
pub trait Transport {
    /// Result type for the transport
    type DataStream: MailDataStream;

    /// Start sending e-mail and return a stream to write the body to
    #[future_is[Sync]]
    async fn send_stream(
        &self,
        envelope: Envelope,
    ) -> Result<Self::DataStream, <Self::DataStream as MailDataStream>::Error>;

    /// Send the email
    #[future_is[Sync]]
    async fn send<R>(
        &self,
        envelope: Envelope,
        message: R,
    ) -> Result<
        <Self::DataStream as MailDataStream>::Output,
        <Self::DataStream as MailDataStream>::Error,
    >
    where
        Self::DataStream: Unpin + Send + Sync,
        <Self::DataStream as MailDataStream>::Error: From<std::io::Error>,
        R: Read + Unpin + Send + Sync,
    {
        let mut stream = self.send_stream(envelope).await?;
        copy(message, &mut stream).await?;
        stream.close().await?;
        stream.result()
    }
}

pub trait MailDataStream: Write {
    type Output;
    type Error;
    /// Return the result of sending the mail.
    /// This should return error if the mail has not been fully dispatched.
    /// In other words, it should fail if the mail data stream hasn't been closed.
    fn result(&self) -> Result<Self::Output, Self::Error>;
}
