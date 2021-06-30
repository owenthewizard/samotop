//! Reference implementation of a mail service
//! simply delivering mail to single maildir directory.
//! Files are named with the transaftion id.

mod delivery;
mod error;
mod stream;
pub use self::{delivery::*, error::*, stream::*};
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
pub struct MaildirTransport {
    path: PathBuf,
}

impl MaildirTransport {
    /// Creates a new transport to the given directory
    pub fn new<P: AsRef<Path>>(path: P) -> MaildirTransport {
        MaildirTransport {
            path: PathBuf::from(path.as_ref()),
        }
    }
}

impl Transport for MaildirTransport {
    type DataStream = MailFile;
    type Error = Error;
    fn send_stream<'s, 'a>(&'s self, envelope: Envelope) -> SyncFuture<'a, Result<MailFile, Error>>
    where
        's: 'a,
    {
        let id = envelope.message_id().to_owned();
        let dir = self.path.clone();

        let mut headers = String::new();
        if let Some(sender) = envelope.from() {
            headers += format!("X-Samotop-From: {}\r\n", sender).as_str();
        }
        for rcpt in envelope.to() {
            headers += format!("X-Samotop-To: {}\r\n", rcpt).as_str();
        }

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
