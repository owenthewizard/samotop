//! The file transport writes the emails to the given directory. The name of the file will be
//! `message_id.txt`.
//! It can be useful for testing purposes, or if you want to keep track of sent messages.
//!

mod error;
pub use self::error::*;
use crate::file::error::{Error, FileResult};
use crate::Envelope;
use crate::MailDataStream;
use crate::Transport;
use async_std::fs::File;
use async_std::io::Write;
use async_std::path::Path;
use futures::io::AsyncWriteExt;
use futures::ready;
use samotop_async_trait::async_trait;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Writes the content and the envelope information to a file.
#[derive(Debug)]
#[cfg_attr(
    feature = "serde-impls",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
pub struct FileTransport {
    path: PathBuf,
}

impl FileTransport {
    /// Creates a new transport to the given directory
    pub fn new<P: AsRef<Path>>(path: P) -> FileTransport {
        FileTransport {
            path: PathBuf::from(path.as_ref()),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
#[cfg_attr(
    feature = "serde-impls",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
struct SerializableEmail {
    envelope: Envelope,
}

#[async_trait]
impl Transport for FileTransport {
    type DataStream = FileStream;
    #[future_is[Sync]]
    async fn send_stream(&self, envelope: Envelope) -> Result<FileStream, Error> {
        let mut file = self.path.clone();
        file.push(format!("{}.json", envelope.message_id()));

        let mut serialized = serde_json::to_string(&SerializableEmail { envelope })?;

        serialized += "\n";

        let mut file = File::create(file).await?;
        file.write_all(serialized.as_bytes()).await?;

        Ok(FileStream {
            file,
            closed: false,
        })
    }
}

#[derive(Debug)]
pub struct FileStream {
    file: File,
    closed: bool,
}

impl MailDataStream for FileStream {
    type Output = ();
    type Error = Error;
    fn result(&self) -> FileResult {
        if self.closed {
            Ok(())
        } else {
            Err(Error::Client("file was not closed properly"))
        }
    }
}

impl Write for FileStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::result::Result<usize, std::io::Error>> {
        Pin::new(&mut self.file).poll_write(cx, buf)
    }
    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::result::Result<(), std::io::Error>> {
        Pin::new(&mut self.file).poll_flush(cx)
    }
    fn poll_close(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::result::Result<(), std::io::Error>> {
        ready!(Pin::new(&mut self.file).poll_close(cx)?);
        self.closed = true;
        Poll::Ready(Ok(()))
    }
}
