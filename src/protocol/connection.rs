use crate::model::io::*;
use crate::model::Result;
use futures::prelude::*;
use futures::ready;
use pin_project::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};

pub trait HasConnection
where
    Self: Sized,
{
    fn with_connection(self, connection: Connection) -> WithConnection<Self> {
        WithConnection {
            stream: self,
            connection,
            connected: false,
            shutdown: false,
        }
    }
}

impl<S> HasConnection for S where S: Stream<Item = Result<ReadControl>> {}

#[pin_project(project=WithConnectionProjection)]
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct WithConnection<S> {
    #[pin]
    stream: S,
    connection: Connection,
    connected: bool,
    shutdown: bool,
}

impl<S> Stream for WithConnection<S>
where
    S: Stream<Item = Result<ReadControl>>,
{
    type Item = S::Item;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let WithConnectionProjection {
            stream,
            connection,
            connected,
            shutdown,
        } = self.project();
        if !*connected {
            trace!("Connected {:?}", connection);
            *connected = true;
            return Poll::Ready(Some(Ok(ReadControl::PeerConnected(connection.clone()))));
        }
        match ready!(stream.poll_next(cx)) {
            None => match *shutdown {
                true => Poll::Ready(None),
                false => {
                    trace!("Disonnected {:?}", connection);
                    *shutdown = true;
                    Poll::Ready(Some(Ok(ReadControl::PeerShutdown)))
                }
            },
            Some(c) => Poll::Ready(Some(c)),
        }
    }
}
