//! Reference implementation of a mail service
//! simply delivering mail to single directory.

mod error;
mod stream;
pub use self::{error::*, stream::*};
use crate::{Envelope, SyncFuture, Transport};
use async_std::{
    fs::{create_dir_all, rename, File},
    io::prelude::WriteExt,
    path::Path,
};
use std::path::PathBuf;

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
        let id = envelope.message_id().to_owned();
        let dir = self.path.clone();

        let mut headers = String::new();
        headers += format!("X-Samotop-From: {:?}\r\n", envelope.from()).as_str();
        headers += format!("X-Samotop-To: {:?}\r\n", envelope.to()).as_str();

        let target_dir = dir.join("new");
        let tmp_dir = dir.join("tmp");
        let target_file = target_dir.join(id.as_str());
        let tmp_file = tmp_dir.join(id.as_str());
        let target = Box::pin(rename(tmp_file.clone(), target_file));
        Box::pin(async move {
            ensure_dir(tmp_dir).await?;
            ensure_dir(target_dir).await?;
            let mut file = File::create(tmp_file).await?;
            file.write_all(headers.as_bytes()).await?;
            Ok(MailFile::new(id, file, target))
        })
    }
}

async fn ensure_dir<P: AsRef<Path>>(dir: P) -> std::io::Result<()> {
    if !dir.as_ref().exists().await {
        create_dir_all(dir).await
    } else {
        Ok(())
    }
}
