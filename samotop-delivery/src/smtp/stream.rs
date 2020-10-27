use crate::smtp::error::Error;
use crate::smtp::response::Response;
use crate::smtp::transport::SmtpConnection;
use crate::smtp::util::SmtpDataCodec;
use crate::smtp::util::SmtpProto;
use crate::MailDataStream;
use async_std::io::Read;
use futures::io::{AsyncWrite as Write, AsyncWriteExt as WriteExt};
use futures::{ready, Future};
use log::{debug, trace};
use potential::Lease;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

/// FIXME: this needs to be gracefully degraded to 7bit if 8bit/utf8 is not available
#[allow(missing_debug_implementations)]
pub struct SmtpDataStream<S> {
    state: State<S>,
}

#[allow(missing_debug_implementations)]
enum State<S> {
    Busy,
    Ready(SmtpDataStreamInner<S>),
    Encoding(Pin<Box<dyn Future<Output = std::io::Result<SmtpDataStreamInner<S>>> + Send + Sync>>),
    Closing(Pin<Box<dyn Future<Output = std::io::Result<Response>> + Send + Sync>>),
    Done(Response),
}

#[allow(missing_debug_implementations)]
struct SmtpDataStreamInner<S> {
    inner: Lease<SmtpConnection<S>>,
    codec: SmtpDataCodec,
    message_id: String,
    timeout: Duration,
}

impl<S> SmtpDataStream<S> {
    pub(crate) fn new(
        inner: Lease<SmtpConnection<S>>,
        message_id: String,
        timeout: Duration,
    ) -> Self {
        SmtpDataStream {
            state: State::Ready(SmtpDataStreamInner {
                inner,
                codec: SmtpDataCodec::new(),
                message_id,
                timeout,
            }),
        }
    }
}

impl<S> MailDataStream for SmtpDataStream<S>
where
    S: Read + Write + Unpin + Sync + Send + 'static,
{
    type Output = Response;
    type Error = Error;
    fn result(&self) -> Result<Self::Output, Self::Error> {
        match self.state {
            State::Done(ref response) => Ok(response.clone()),
            _ => Err(Error::Client("Mail sending was not completed properly")),
        }
    }
}

impl<S> Write for SmtpDataStream<S>
where
    S: Read + Write + Unpin + Sync + Send + 'static,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        trace!("poll_write {} bytes", buf.len());
        loop {
            break match std::mem::replace(&mut self.state, State::Busy) {
                State::Ready(SmtpDataStreamInner {
                    mut inner,
                    mut codec,
                    message_id,
                    timeout,
                }) => {
                    let len = buf.len();
                    let buf = Vec::from(buf);
                    let fut = async move {
                        codec.encode(&buf[..], &mut inner.stream).await?;
                        Ok(SmtpDataStreamInner {
                            inner,
                            codec,
                            message_id,
                            timeout,
                        })
                    };
                    self.state = State::Encoding(Box::pin(fut));
                    Poll::Ready(Ok(len))
                }
                otherwise => {
                    self.state = otherwise;
                    ready!(self.as_mut().poll_flush(cx))?;
                    continue;
                }
            };
        }
    }
    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::result::Result<(), std::io::Error>> {
        trace!("poll_flush");
        loop {
            break match self.state {
                State::Ready(ref mut inner) => Pin::new(&mut inner.inner.stream).poll_flush(cx),
                State::Encoding(ref mut fut) => {
                    let inner = ready!(fut.as_mut().poll(cx))?;
                    self.state = State::Ready(inner);
                    continue;
                }
                State::Closing(ref mut fut) => {
                    let response = ready!(fut.as_mut().poll(cx))?;
                    self.state = State::Done(response);
                    continue;
                }
                State::Done(_) => Poll::Ready(Ok(())),
                State::Busy => Poll::Ready(Err(broken())),
            };
        }
    }
    fn poll_close(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::result::Result<(), std::io::Error>> {
        // Defer close so that the connection can be reused.
        // Lease will send the inner client back on drop.
        // Here we take care of closing the stream with final dot
        // and reading the response
        trace!("poll_close");
        loop {
            break match std::mem::replace(&mut self.state, State::Busy) {
                State::Ready(SmtpDataStreamInner {
                    mut inner,
                    mut codec,
                    message_id,
                    timeout,
                }) => {
                    let fut = async move {
                        // write final dot
                        codec.encode(&[][..], &mut inner.stream).await?;
                        // make sure all is in before reading response
                        inner.stream.flush().await?;

                        // collect response
                        trace!("data sent, waiting for confirmation");
                        let mut client = SmtpProto::new(Pin::new(&mut inner.stream));
                        let response = client
                            .read_data_sent_response(timeout)
                            .await
                            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

                        // Log the message
                        debug!("{}: status=sent ({:?})", message_id, response);

                        Ok(response)
                    };
                    self.state = State::Closing(Box::pin(fut));
                    continue;
                }
                otherwise @ State::Encoding(_) | otherwise @ State::Closing(_) => {
                    self.state = otherwise;
                    ready!(self.as_mut().poll_flush(cx))?;
                    continue;
                }
                otherwise @ State::Done(_) | otherwise @ State::Busy => {
                    self.state = otherwise;
                    self.as_mut().poll_flush(cx)
                }
            };
        }
    }
}

fn broken() -> std::io::Error {
    std::io::Error::from(std::io::ErrorKind::NotConnected)
}
