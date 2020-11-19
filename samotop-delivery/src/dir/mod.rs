//! Reference implementation of a mail service
//! simply delivering mail to single directory.

mod error;
mod stream;
pub use self::{error::*, stream::*};
use crate::{Envelope, SyncFuture, Transport};
use async_std::{
    fs::{create_dir_all, rename, File},
    path::Path,
};
use bytes::BytesMut;
use futures::{future::TryFutureExt, ready, Future};
use pin_project::pin_project;
use std::{
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
};

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

impl Transport for FileTransport {
    type DataStream = MailFile;
    fn send_stream<'s, 'a>(&'s self, envelope: Envelope) -> SyncFuture<'a, Result<MailFile, Error>>
    where
        's: 'a,
    {
        let mut file = self.path.clone();
        file.push(format!("{}.json", envelope.message_id()));

        Box::pin(CreateMailFile::new(&self.path, envelope))
    }
}

#[pin_project]
pub struct CreateMailFile {
    // TODO: Refactor complex type
    #[allow(clippy::type_complexity)]
    stage2: Option<(
        BytesMut,
        String,
        Pin<Box<dyn Future<Output = std::io::Result<()>> + Send + Sync + 'static>>,
    )>,
    file: Pin<Box<dyn Future<Output = std::io::Result<File>> + Send + Sync + 'static>>,
}

impl std::fmt::Debug for CreateMailFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreateMailFile").finish()
    }
}

impl CreateMailFile {
    pub fn new<D: AsRef<Path>>(dir: D, envelope: Envelope) -> Self {
        let mut headers = BytesMut::new();
        headers.extend(format!("X-Samotop-From: {:?}\r\n", envelope.from()).bytes());
        headers.extend(format!("X-Samotop-To: {:?}\r\n", envelope.to()).bytes());

        let target_dir = dir.as_ref().join("new");
        let tmp_dir = dir.as_ref().join("tmp");
        let target_file = target_dir.join(envelope.message_id());
        let tmp_file = tmp_dir.join(envelope.message_id());
        let target = Box::pin(rename(tmp_file.clone(), target_file));
        let file = Box::pin(
            ensure_dir(tmp_dir)
                .and_then(move |_| ensure_dir(target_dir))
                .and_then(move |_| File::create(tmp_file)),
        );

        Self {
            stage2: Some((headers, envelope.message_id().to_owned(), target)),
            file,
        }
    }
}

async fn ensure_dir<P: AsRef<Path>>(dir: P) -> std::io::Result<()> {
    if !dir.as_ref().exists().await {
        create_dir_all(dir).await
    } else {
        Ok(())
    }
}

impl Future for CreateMailFile {
    type Output = Result<MailFile, Error>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match ready!(Pin::new(&mut self.file).poll(cx)) {
            Ok(file) => {
                if let Some((buffer, id, target)) = self.stage2.take() {
                    let mailfile = MailFile::new(id, file, buffer, target);
                    Poll::Ready(Ok(mailfile))
                } else {
                    error!("No buffer/id. Perhaps the future has been polled after Poll::Ready");
                    Poll::Ready(Err(Error::Client("future is empty")))
                }
            }
            Err(e) => {
                error!("Could not create mail file: {:?}", e);
                Poll::Ready(Err(Error::Io(e)))
            }
        }
    }
}
