use super::*;
use crate::{Envelope, SyncFuture, Transport};
use async_std::{
    fs::{create_dir_all, OpenOptions},
    path::Path,
};
use futures::AsyncWriteExt;
use lozizol::model::Sequence;
use potential::Potential;
use std::path::PathBuf;
use uuid::Uuid;

/// This transport logs the message envelope and returns the given response
#[derive(Debug)]
pub struct JournalTransport {
    dir: PathBuf,
    bucket: Potential<Bucket>,
    max_size: usize,
}

impl Default for JournalTransport {
    /// Creates a new transport that stores a journal in current folder
    fn default() -> Self {
        Self::new(".")
    }
}

impl JournalTransport {
    /// Creates a new transport that stores a journal in the given folder
    pub fn new(dir: impl Into<PathBuf>) -> JournalTransport {
        JournalTransport {
            dir: dir.into(),
            bucket: Potential::empty(),
            max_size: 2_000_000_000,
        }
    }
    async fn make_bucket(&self) -> JournalResult<Bucket> {
        let sequence_id = Uuid::new_v4().to_hyphenated().to_string();
        ensure_dir(&self.dir).await?;
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(self.dir.join(sequence_id.as_str()))
            .await?;

        let mut sequence = Sequence::new();
        sequence.set_id(sequence_id)?;
        Ok(Bucket::new(file, sequence))
    }
}

impl Transport for JournalTransport {
    type DataStream = JournalStream;
    fn send_stream<'life1, 'async_trait>(
        &'life1 self,
        envelope: Envelope,
    ) -> SyncFuture<JournalResult<JournalStream>>
    where
        'life1: 'async_trait,
    {
        Box::pin(async move {
            let bucket = loop {
                break match self.bucket.lease().await {
                    Ok(bucket) => {
                        if bucket.written > self.max_size {
                            let mut bucket = bucket.steal();
                            bucket.write.close().await?;
                            continue;
                        } else {
                            bucket
                        }
                    }
                    Err(gone) => gone.set(self.make_bucket().await?),
                };
            };

            Ok(JournalStream::new(bucket, envelope))
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
