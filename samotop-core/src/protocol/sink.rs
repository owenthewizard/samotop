use crate::common::*;
use std::marker::PhantomData;

pub trait SinkFutureExt<I>: Sink<I> {
    fn send(self: Pin<Box<Self>>, item: I) -> SendItem<Self, I>
    where
        I: Unpin,
    {
        SendItem::new(self, item)
    }
    fn close(self: Pin<Box<Self>>) -> CloseSink<Self, I>
    where
        I: Unpin,
    {
        CloseSink::close(self)
    }
}
impl<T: Sink<I> + ?Sized, I> SinkFutureExt<I> for T {}

pub struct CloseSink<S: ?Sized, I> {
    sink: Pin<Box<S>>,
    phantom: PhantomData<I>,
}
impl<S: Sink<I> + ?Sized, I: Unpin> CloseSink<S, I> {
    pub fn close(sink: Pin<Box<S>>) -> Self {
        CloseSink {
            sink: sink,
            phantom: PhantomData,
        }
    }
}
impl<S: Sink<I> + ?Sized, I: Unpin> Future for CloseSink<S, I> {
    type Output = std::result::Result<(), S::Error>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.sink.as_mut().poll_close(cx)
    }
}

pub struct SendItem<S: ?Sized, I> {
    item: Option<I>,
    sink: Option<Pin<Box<S>>>,
}
impl<S: Sink<I> + ?Sized, I: Unpin> SendItem<S, I> {
    pub fn new(sink: Pin<Box<S>>, item: I) -> Self {
        SendItem {
            sink: Some(sink),
            item: Some(item),
        }
    }
}
impl<S: Sink<I> + ?Sized, I: Unpin> Future for SendItem<S, I> {
    type Output = std::result::Result<Pin<Box<S>>, S::Error>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let mut sink = self
                .sink
                .take()
                .expect("sink must be set, called after error?");
            if let Some(item) = self.item.take() {
                if sink.as_mut().poll_ready(cx)?.is_ready() {
                    sink.as_mut().start_send(item)?;
                    self.sink = Some(sink);
                } else {
                    self.sink = Some(sink);
                    break Poll::Pending;
                }
            } else {
                if sink.as_mut().poll_flush(cx)?.is_ready() {
                    break Poll::Ready(Ok(sink));
                } else {
                    self.sink = Some(sink);
                    break Poll::Pending;
                }
            }
        }
    }
}
