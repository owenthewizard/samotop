use crate::{
    journal::{JournalError, JournalResult},
    EmailAddress, Envelope, MailDataStream,
};
use async_std::{
    fs::{create_dir_all, OpenOptions},
    io,
    path::{Path, PathBuf},
};
use futures::AsyncWriteExt;
use lozizol::model::{Sequence, Vuint};
use potential::{Gone, Lease, Potential};
use samotop_core::common::*;
use uuid::Uuid;

#[derive(Debug)]
pub struct JournalStream {
    state: State,
    dir: PathBuf,
    max_bucket_size: usize,
    envelope: Envelope,
    buffer: Vec<u8>,
    block_size: usize,
    blocks: Vec<(Uuid, usize)>,
    result: JournalResult<()>,
}
impl JournalStream {
    pub(crate) fn new(dir: PathBuf, bucket: Arc<Potential<Bucket>>, envelope: Envelope) -> Self {
        JournalStream {
            state: State::Ready(bucket),
            dir,
            max_bucket_size: 2_000_000_000,
            envelope,
            buffer: vec![],
            block_size: 1_000_000,
            blocks: vec![],
            result: Ok(()),
        }
    }
    fn buffer_capacity(&self) -> usize {
        self.block_size - self.buffer.len()
    }
}
impl MailDataStream for JournalStream {
    type Output = ();
    type Error = JournalError;
    fn result(&mut self) -> JournalResult<()> {
        std::mem::replace(&mut self.result, Ok(()))
    }
}

impl io::Write for JournalStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        info!("Writing {} bytes", buf.len());

        let mut len = usize::min(buf.len(), self.buffer_capacity());

        if len == 0 {
            if self.as_mut().poll_flush(cx)?.is_pending() {
                return Poll::Pending;
            } else {
                len = usize::min(buf.len(), self.buffer_capacity())
            }
        }

        self.buffer.extend_from_slice(&buf[..len]);

        Poll::Ready(Ok(len))
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        info!("Flushing");
        loop {
            break match std::mem::take(&mut self.state) {
                State::Ready(bucket) => {
                    let dir = self.dir.clone();
                    let max_bucket_size = self.max_bucket_size;
                    let buf = std::mem::take(&mut self.buffer);
                    let bucket_copy = bucket.clone();
                    let fut = Box::pin(async move {
                        use std::ops::DerefMut;
                        let mut lease =
                            get_bucket(bucket_copy.lease().await, dir, max_bucket_size).await?;
                        let Bucket {
                            ref mut sequence,
                            ref mut write,
                            ..
                        } = lease.deref_mut();
                        let block = if buf.is_empty() {
                            write.flush().await?;
                            None
                        } else {
                            let mut entry = lozizol::task::encode::entry(
                                sequence,
                                write,
                                "urn:samotop:block-v1",
                                &buf.len(),
                            )
                            .await?;
                            // copy flushes
                            io::copy(buf.as_slice(), &mut entry).await?;
                            let position = entry.position().to_owned();
                            let seq_id = sequence.id().parse().expect("valid uuid");
                            Some((seq_id, position))
                        };

                        Ok((bucket, block))
                    });
                    self.state = State::Encoding(fut);
                    continue;
                }
                State::Encoding(mut fut) => match fut.as_mut().poll(cx) {
                    Poll::Ready(Ok((bucket, block))) => {
                        // record the block reference to a sequence position
                        if let Some((sequence_id, position)) = block {
                            self.blocks.push((sequence_id, position));
                        }
                        self.state = State::Ready(bucket);
                        continue;
                    }
                    Poll::Pending => {
                        self.state = State::Encoding(fut);
                        Poll::Pending
                    }
                    Poll::Ready(Err(e)) => {
                        Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e)))
                    }
                },
                State::Closing(_) => {
                    panic!("Flushing in closing state")
                }
                State::Invalid => {
                    panic!("Flushing in closed/error state")
                }
            };
        }
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        info!("Closing");
        // close writes by setting buffer size to 0
        self.block_size = 0;
        let addr_to_vec = |addr: &EmailAddress| {
            let addr: &[u8] = addr.as_ref();
            let len = Vuint::from(&addr.len());
            let len = len.as_ref();
            let mut vec = Vec::with_capacity(addr.len() + len.len());
            vec.extend_from_slice(len);
            vec.extend_from_slice(addr);
            vec
        };
        loop {
            break match std::mem::take(&mut self.state) {
                State::Ready(bucket) => {
                    if !self.buffer.is_empty() {
                        if self.as_mut().poll_flush(cx).is_pending() {
                            self.state = State::Ready(bucket);
                            return Poll::Pending;
                        }
                    }

                    if self.blocks.is_empty() {
                        let bucket = bucket.clone();
                        self.state = State::Closing(Box::pin(async move {
                            match bucket.lease().await {
                                Err(_) => Ok(()),
                                Ok(lease) => Ok(lease.steal().write.close().await?),
                            }
                        }));
                        continue;
                    } else {
                        let from = self
                            .envelope
                            .from()
                            .map(addr_to_vec)
                            // 0 length means none
                            .unwrap_or_else(|| vec![0]);
                        let rcpts: Vec<Vec<u8>> =
                            self.envelope.to().iter().map(addr_to_vec).collect();
                        let blocks = std::mem::take(&mut self.blocks).into_iter().fold(
                            vec![],
                            |mut buf, (seq, pos)| {
                                buf.extend_from_slice(seq.as_bytes());
                                buf.extend_from_slice(Vuint::from(pos).as_ref());
                                buf
                            },
                        );
                        let dir = self.dir.clone();
                        let max_bucket_size = self.max_bucket_size;
                        let bucket = bucket.clone();
                        let fut = Box::pin(async move {
                            use std::ops::DerefMut;
                            let mut bucket =
                                get_bucket(bucket.lease().await, dir, max_bucket_size).await?;
                            let Bucket {
                                ref mut sequence,
                                ref mut write,
                                ..
                            } = bucket.deref_mut();

                            for rcpt in rcpts {
                                let len = rcpt.len() + blocks.len() + from.len();
                                let mut entry = lozizol::task::encode::entry(
                                    sequence,
                                    &mut *write,
                                    "urn:samotop:test",
                                    &len,
                                )
                                .await?;

                                io::copy(rcpt.as_slice(), &mut entry).await?;
                                io::copy(from.as_slice(), &mut entry).await?;
                                io::copy(blocks.as_slice(), &mut entry).await?;
                                entry.flush().await?;
                            }

                            Ok(())
                        });
                        self.state = State::Closing(fut);
                        continue;
                    }
                }
                State::Encoding(fut) => {
                    self.state = State::Encoding(fut);
                    match self.as_mut().poll_flush(cx)? {
                        Poll::Ready(()) => continue,
                        Poll::Pending => Poll::Pending,
                    }
                }
                State::Closing(mut fut) => match fut.as_mut().poll(cx) {
                    Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
                    Poll::Pending => {
                        self.state = State::Closing(fut);
                        Poll::Pending
                    }
                    Poll::Ready(Err(e)) => {
                        Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e)))
                    }
                },
                State::Invalid => Poll::Ready(Ok(())),
            };
        }
    }
}

enum State {
    Ready(Arc<Potential<Bucket>>),
    Encoding(S3Fut<JournalResult<(Arc<Potential<Bucket>>, Option<(Uuid, usize)>)>>),
    Closing(S3Fut<JournalResult<()>>),
    Invalid,
}
impl Default for State {
    fn default() -> Self {
        State::Invalid
    }
}
impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            State::Ready(l) => f.debug_tuple("Ready").field(&l).finish(),
            State::Encoding(_) => f.debug_tuple("Encoding").field(&"...").finish(),
            State::Closing(_) => f.debug_tuple("Closing").field(&"...").finish(),
            State::Invalid => f.debug_tuple("Invalid").finish(),
        }
    }
}

async fn get_bucket<P: AsRef<Path>>(
    potential: std::result::Result<Lease<Bucket>, Gone<Bucket>>,
    dir: P,
    max_size: usize,
) -> JournalResult<Lease<Bucket>> {
    match potential {
        Ok(mut bucket) => {
            if bucket.written > max_size {
                bucket
                    .replace(create_bucket(dir).await?)
                    .write
                    .close()
                    .await?;
            }
            Ok(bucket)
        }
        Err(gone) => Ok(gone.set(create_bucket(dir).await?)),
    }
}
async fn create_bucket<P: AsRef<Path>>(dir: P) -> JournalResult<Bucket> {
    let sequence_id = Uuid::new_v4().to_hyphenated().to_string();
    ensure_dir(&dir).await?;
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(dir.as_ref().join(sequence_id.as_str()))
        .await?;

    let mut sequence = Sequence::new();
    sequence.set_id(sequence_id)?;
    Ok(Bucket::new(file, sequence))
}
async fn ensure_dir<P: AsRef<Path>>(dir: P) -> std::io::Result<()> {
    if !dir.as_ref().exists().await {
        create_dir_all(dir).await
    } else {
        Ok(())
    }
}

pub(crate) trait BucketWrite: io::Write + Send + Sync + Unpin + 'static {}
impl<T> BucketWrite for T where T: io::Write + Send + Sync + Unpin + 'static {}

pub(crate) struct Bucket {
    pub(crate) write: Box<dyn BucketWrite>,
    sequence: Sequence,
    pub(crate) written: usize,
}
impl Bucket {
    pub fn new<W>(write: W, sequence: Sequence) -> Self
    where
        W: BucketWrite,
    {
        Bucket {
            write: Box::new(write),
            sequence,
            written: 0,
        }
    }
}
impl fmt::Debug for Bucket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(stringify!(Bucket)).finish()
    }
}
