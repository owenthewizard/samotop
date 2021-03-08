use crate::{
    journal::{JournalError, JournalResult},
    EmailAddress, Envelope, MailDataStream,
};
use async_std::io;
use futures::AsyncWriteExt;
use lozizol::model::{Sequence, Vuint};
use potential::Lease;
use samotop_core::common::*;
use uuid::Uuid;

#[derive(Debug)]
pub struct JournalStream {
    state: State,
    envelope: Envelope,
    buffer: Vec<u8>,
    block_size: usize,
    blocks: Vec<(Uuid, usize)>,
}
impl JournalStream {
    pub(crate) fn new(bucket: Lease<Bucket>, envelope: Envelope) -> Self {
        JournalStream {
            state: State::Ready(bucket),
            envelope,
            buffer: vec![],
            block_size: 1_000_000,
            blocks: vec![],
        }
    }
    fn buffer_capacity(&self) -> usize {
        self.block_size - self.buffer.len()
    }
}
impl MailDataStream for JournalStream {
    type Output = ();
    type Error = JournalError;
    fn result(&self) -> JournalResult<()> {
        todo!()
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
                State::Ready(mut bucket) => {
                    if Pin::new(&mut bucket.write).poll_flush(cx)?.is_pending() {
                        self.state = State::Ready(bucket);
                        Poll::Pending
                    } else if self.buffer.is_empty() {
                        self.state = State::Ready(bucket);
                        Poll::Ready(Ok(()))
                    } else {
                        let buf = std::mem::take(&mut self.buffer);
                        let fut = Box::pin(async move {
                            use std::ops::DerefMut;
                            let Bucket {
                                ref mut sequence,
                                ref mut write,
                                ..
                            } = bucket.deref_mut();
                            let mut entry = lozizol::task::encode::entry(
                                sequence,
                                write,
                                "urn:samotop:block-v1",
                                &buf.len(),
                            )
                            .await?;

                            io::copy(buf.as_slice(), &mut entry).await?;
                            let position = entry.position().to_owned();
                            drop(entry);
                            Ok((bucket, position))
                        });
                        self.state = State::Encoding(fut);
                        continue;
                    }
                }
                State::Encoding(mut fut) => match fut.as_mut().poll(cx) {
                    Poll::Ready(Ok((bucket, position))) => {
                        // record the block reference to a sequence position
                        let seq_id = bucket.sequence.id().parse().expect("valid uuid");
                        self.blocks.push((seq_id, position));
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
                State::Ready(mut bucket) => {
                    if !self.buffer.is_empty() {
                        self.state = State::Ready(bucket);
                        self.poll_flush(cx)
                    } else if self.blocks.is_empty() {
                        Pin::new(&mut bucket.write).poll_close(cx)
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

                        let fut = Box::pin(async move {
                            use std::ops::DerefMut;
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
    Ready(Lease<Bucket>),
    Encoding(S3Fut<JournalResult<(Lease<Bucket>, usize)>>),
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
