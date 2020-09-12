use crate::common::*;
use crate::model::io::WriteControl;
use crate::model::Result;

pub trait FuseShutdown
where
    Self: Sized,
{
    fn fuse_shutdown(self) -> Fuse<Self> {
        Fuse {
            stream: self,
            trip: false,
        }
    }
}

impl<S> FuseShutdown for S where S: Stream {}

#[pin_project(project=FuseProjection)]
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct Fuse<S> {
    #[pin]
    stream: S,
    trip: bool,
}

impl<S> Stream for Fuse<S>
where
    S: Stream<Item = Result<WriteControl>>,
{
    type Item = S::Item;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let FuseProjection { stream, trip } = self.project();

        if *trip {
            return Poll::Ready(None);
        }
        let item = ready!(stream.poll_next(cx));
        if let Some(Ok(WriteControl::Shutdown(_))) = item {
            *trip = true;
        }
        Poll::Ready(item)
    }
}
