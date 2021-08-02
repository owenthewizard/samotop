/*!

# Mail dispatch abstraction

samotop-delivery is a set of transports to deliver mail to,
notably to SMTP/LMTP, but also maildir... It is used in Samotop
as a dispatch solution for incoming mail, but you can use it to send mail, too.

## Example
```rust
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;
use samotop_delivery::prelude::{
    Envelope, SmtpClient, Transport,
};
async fn smtp_transport_simple() -> Result<()> {
    let envelope = Envelope::new(
            Some("user@localhost".parse()?),
            vec!["root@localhost".parse()?],
            "id".to_string(),
        )?;
    let message = "From: user@localhost\r\n\
                    Content-Type: text/plain\r\n\
                    \r\n\
                    Hello example"
                    .as_bytes();
    let client = SmtpClient::new("127.0.0.1:2525")?;

    // Create a client, connect and send
    client.connect_and_send(envelope, message).await?;
    Ok(())
}
```

# Features
 - [x] Do it SMTP style:
    - [x] Speak SMTP
    - [x] Speak LMTP
    - [x] Connect over TCP
    - [x] Connect over Unix sockets
    - [x] Connect to a Child process IO
    - [x] TLS support on all connections
    - [x] Reuse established connections
 - [x] Do it locally:
    - [x] Write mail to a MailDir
    - [x] Write mail to lozizol journal
    - [ ] Write mail to an MBox file - contributions welcome
    - [x] Write mail to a single dir - fit for debug only
 - [x] Popular integrations:
    - [x] Send mail with sendmail

LMTP on Unix socket enables wide range of local delivery integrations, dovecot or postfix for instance. Some mail delivery programs speak LMTP, too.

# Credits

This is a fork of [async-smtp](https://github.com/async-email/async-smtp/releases/tag/v0.3.4)
from the awesome [delta.chat](https://delta.chat) project.

*/

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
use async_std::io::{copy, Write};
use samotop_core::common::*;
use std::{fmt, pin::Pin};

pub type SyncFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Sync + Send + 'a>>;
type SendResult<T> = std::result::Result<<T as Transport>::DataStream, <T as Transport>::Error>;

/// Transport method for emails
pub trait Transport: std::fmt::Debug {
    /// Result type for the transport
    type DataStream: MailDataStream;
    type Error;

    /// Start sending e-mail and return a stream to write the body to
    fn send_stream<'s, 'a>(&'s self, envelope: Envelope) -> SyncFuture<'a, SendResult<Self>>
    where
        's: 'a;

    /// Send the email
    fn send<'s, 'r, 'a, R>(
        &'s self,
        envelope: Envelope,
        message: R,
    ) -> SyncFuture<'a, SendResult<Self>>
    where
        Self::DataStream: Unpin + Send + Sync,
        Self::Error: From<std::io::Error>,
        R: io::Read + Unpin + Send + Sync + 'r,
        's: 'a,
        'r: 'a,
    {
        let stream = self.send_stream(envelope);
        Box::pin(async move {
            let mut stream = stream.await?;
            copy(message, &mut stream).await?;
            poll_fn(|cx| Pin::new(&mut stream).poll_close(cx)).await?;
            Ok(stream)
        })
    }
}

pub trait MailDataStream: fmt::Debug + io::Write {
    /// Return the result of sending the mail.
    /// This should return false if the mail has not been fully dispatched.
    /// In other words, the test should fail if the mail data stream hasn't been closed.
    fn is_done(&self) -> bool;
}
