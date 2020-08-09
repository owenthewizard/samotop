use crate::common::*;
use futures::stream::{Forward, SplitSink, SplitStream};

/**
 * Any `Streem` Self can be passed 'through' another `Sink+Stream` J so that
 * items from stream Self are forwarded into sink J and the resulting
 * stream picks items from stream J
 */
pub trait IntoThroughStream: Sized {
    /**
     * pass `Stream` Self through another `Sink+Stream` J so that
     * items from stream Self are forwarded into sink J and the resulting
     * stream picks items from stream J
     */
    fn through<H, T, E>(
        self,
        junction: H,
    ) -> ThroughStream<Forward<Self, SplitSink<H, T>>, SplitStream<H>>
    where
        Self: Stream<Item = std::result::Result<T, E>>,
        H: Sink<T, Error = E>,
        H: Stream,
    {
        let (sink, outbound) = junction.split();
        let forward = Some(self.forward(sink));
        ThroughStream { forward, outbound }
    }
}

impl<S> IntoThroughStream for S where S: Stream {}

#[pin_project(project=ThroughProjection)]
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct ThroughStream<F, O> {
    #[pin]
    forward: Option<F>,
    #[pin]
    outbound: O,
}

impl<F, O, T, E, I, IE> Stream for ThroughStream<F, O>
where
    F: Future<Output = std::result::Result<T, E>>,
    O: Stream<Item = std::result::Result<I, IE>>,
    IE: From<E>,
    I: std::fmt::Debug,
    IE: std::fmt::Debug,
    T: std::fmt::Debug,
    E: std::fmt::Debug,
{
    type Item = O::Item;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        trace!("Polling next");

        let ThroughProjection {
            mut forward,
            mut outbound,
        } = self.as_mut().project();

        if let Some(ref mut fw) = forward.as_mut().as_pin_mut() {
            if let Poll::Ready(finished) = fw.as_mut().poll(cx)? {
                trace!("Forwarding finished {:?}", finished);
                forward.set(None);
            }
        }

        let out = outbound.as_mut().poll_next(cx);
        trace!("Outbound: {:?}", out);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Error, Result};
    use futures::stream;
    use futures::stream::StreamExt;
    use futures_await_test::async_test;
    use pin_project::pin_project;

    fn fail<T>(e: &str) -> impl Stream<Item = Result<T>> {
        stream::once(future::ready(Err(e.into())))
    }
    fn once<T>(i: T) -> impl Stream<Item = Result<T>> {
        stream::once(future::ready(Ok(i)))
    }
    fn empty<T>() -> impl Stream<Item = Result<T>> {
        stream::empty()
    }

    #[async_test]
    async fn through_poll_handles_empty_stream() {
        let result = empty::<()>().through(DummyPassOne::default()).next().await;
        assert!(result.is_none());
    }

    #[async_test]
    async fn through_poll_picks_single_item_stream() {
        let result = once(()).through(DummyPassOne::default()).next().await;
        assert_eq!(result.unwrap().unwrap(), ());
    }

    #[async_test]
    async fn through_poll_picks_error_from_dummy() {
        let result = empty::<()>()
            .through(DummyPassOne::error("strange"))
            .next()
            .await;
        assert!(result.unwrap().is_err());
    }

    #[async_test]
    async fn through_poll_picks_error_from_stream() {
        let result = fail::<()>("strange")
            .through(DummyPassOne::default())
            .next()
            .await;
        assert!(result.unwrap().is_err());
    }

    #[async_test]
    async fn through_poll_picks_dummy_first() {
        let result = once(1u8).through(DummyPassOne::item(2)).next().await;
        assert_eq!(result.unwrap().unwrap(), 2);
    }

    #[async_test]
    async fn through_poll_picks_item_from_dummy() {
        let result = empty().through(DummyPassOne::item(5)).next().await;
        assert_eq!(result.unwrap().unwrap(), 5);
    }

    #[pin_project]
    #[derive(Default)]
    struct DummyPassOne<I> {
        pub item: Option<I>,
        pub error: Option<Error>,
    }
    impl<I> DummyPassOne<I> {
        pub fn item(item: I) -> Self {
            Self {
                item: Some(item),
                error: None,
            }
        }
        pub fn error(error: impl Into<Error>) -> Self {
            Self {
                item: None,
                error: Some(error.into()),
            }
        }
    }

    impl<I> Stream for DummyPassOne<I> {
        type Item = Result<I>;
        fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            println!("poll has: {}", self.item.is_some());
            let projection = self.project();
            match projection.item.take() {
                Some(item) => Poll::Ready(Some(Ok(item))),
                None => match projection.error.take() {
                    Some(e) => Poll::Ready(Some(Err(e))),
                    None => Poll::Ready(None),
                },
            }
        }
    }
    impl<I> Sink<I> for DummyPassOne<I> {
        type Error = Error;
        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
            self.poll_ready(cx)
        }
        fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
            self.poll_flush(cx)
        }
        fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
            println!("poll_ready has: {}", self.item.is_some());
            match self.item {
                Some(_) => Poll::Pending,
                None => Poll::Ready(Ok(())),
            }
        }
        fn start_send(self: Pin<&mut Self>, item: I) -> Result<()> {
            println!("start_send has: {}", self.item.is_some());
            if self.item.is_none() {
                *self.project().item = Some(item);
                Ok(())
            } else {
                panic!("called start_send while not ready!")
            }
        }
    }
}
