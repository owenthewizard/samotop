use super::error::*;
use super::*;
use crate::{Envelope, SyncFuture, Transport};
use potential::Potential;
use std::{path::PathBuf, sync::Arc};

/// This transport logs the message envelope and returns the given response
#[derive(Debug)]
pub struct JournalTransport {
    dir: PathBuf,
    bucket: Arc<Potential<Bucket>>,
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
            bucket: Arc::new(Potential::empty()),
            max_size: 2_000_000_000,
        }
    }
}

impl Transport for JournalTransport {
    type DataStream = JournalStream;
    type Error = Error;
    fn send_stream<'life1, 'async_trait>(
        &'life1 self,
        envelope: Envelope,
    ) -> SyncFuture<Result<JournalStream, Error>>
    where
        'life1: 'async_trait,
    {
        Box::pin(async move {
            Ok(JournalStream::new(
                self.dir.as_path().into(),
                self.bucket.clone(),
                envelope,
            ))
        })
    }
}
