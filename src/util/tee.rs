use futures::prelude::*;
use futures::stream::{Forward, SplitSink, SplitStream};
use futures::Async::*;
use util::futu::*;
/**
 * Any `Streem` Self can be 'tee'd into another `Sink+Stream` J so that
 * items from stream A are forwarded into sink J and the resulting
 * stream picks items from stream J
 */
pub trait IntoTee: Sized {
    /**
     * tee `Stream` Self into another `Sink+Stream` J so that
     * items from stream A are forwarded into sink J and the resulting
     * stream picks items from stream J
     */
    fn tee<H>(self, junction: H) -> Tee<Forward<Self, SplitSink<H>>, SplitStream<H>>
    where
        Self: Stream,
        H: Sink<SinkItem = Self::Item, SinkError = Self::Error>,
        H: Stream<Error = Self::Error>,
    {
        let (sink, outbound) = junction.split();
        let forward = self.forward(sink);
        Tee { forward, outbound }
    }
}

impl<S> IntoTee for S
where
    S: Stream,
{
}

pub struct Tee<F, O> {
    forward: F,
    outbound: O,
}

impl<F, O> Stream for Tee<F, O>
where
    F: Future,
    O: Stream<Error = F::Error>,
{
    type Item = O::Item;
    type Error = F::Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.forward.poll() {
            Err(e) => Err(e),
            Ok(Ready(_)) => match self.outbound.poll() {
                Err(e) => Err(e),
                Ok(Ready(Some(i))) => ok(i),
                Ok(Ready(None)) => none(),
                Ok(NotReady) => pending(),
            },
            Ok(NotReady) => match self.outbound.poll() {
                Err(e) => Err(e),
                Ok(Ready(Some(i))) => ok(i),
                Ok(Ready(None)) => pending(),
                Ok(NotReady) => pending(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;
    use futures::StartSend;

    #[test]
    fn tee_poll_handles_empty_stream() {
        let result = stream::empty::<(), ()>()
            .tee(DummyPassOne::default())
            .poll();
        assert_eq!(result, Ok(Ready(None)));
    }

    #[test]
    fn tee_poll_picks_single_item_stream() {
        let result = stream::once::<(), ()>(Ok(()))
            .tee(DummyPassOne::default())
            .poll();
        assert_eq!(result, Ok(Ready(Some(()))));
    }

    #[test]
    fn tee_poll_picks_error_from_dummy() {
        let result = stream::empty::<(), ()>()
            .tee(DummyPassOne::error(()))
            .poll();
        assert_eq!(result, Err(()));
    }

    #[test]
    fn tee_poll_picks_error_from_stream() {
        let result = stream::once::<(), ()>(Err(()))
            .tee(DummyPassOne::default())
            .poll();
        assert_eq!(result, Err(()));
    }

    #[test]
    fn tee_poll_picks_dummy_first() {
        let result = stream::once::<u8, ()>(Ok(1))
            .tee(DummyPassOne::item(2))
            .poll();
        assert_eq!(result, Ok(Ready(Some(2))));
    }

    #[test]
    fn tee_poll_picks_item_from_dummy() {
        let result = stream::empty::<(), ()>().tee(DummyPassOne::item(())).poll();
        assert_eq!(result, Ok(Ready(Some(()))));
    }

    #[derive(Default)]
    struct DummyPassOne<I, E> {
        pub item: Option<I>,
        pub error: Option<E>,
    }
    impl<I, E> DummyPassOne<I, E> {
        pub fn item(item: I) -> Self {
            Self {
                item: Some(item),
                error: None,
            }
        }
        pub fn error(error: E) -> Self {
            Self {
                item: None,
                error: Some(error),
            }
        }
    }

    impl<I, E> Stream for DummyPassOne<I, E> {
        type Item = I;
        type Error = E;
        fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
            println!("poll has: {}", self.item.is_some());
            match self.item.take() {
                Some(item) => ok(item),
                None => match self.error.take() {
                    Some(e) => Err(e),
                    None => none(),
                },
            }
        }
    }
    impl<I, E> Sink for DummyPassOne<I, E> {
        type SinkItem = I;
        type SinkError = E;
        fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
            println!("poll_complete has: {}", self.item.is_some());
            match self.item {
                Some(_) => Ok(NotReady),
                None => Ok(Ready(())),
            }
        }
        fn start_send(
            &mut self,
            item: Self::SinkItem,
        ) -> StartSend<Self::SinkItem, Self::SinkError> {
            println!("start_send has: {}", self.item.is_some());
            if self.item.is_none() {
                self.item = Some(item);
                Ok(AsyncSink::Ready)
            } else if self.error.is_none() {
                Ok(AsyncSink::NotReady(item))
            } else {
                Err(self.error.take().unwrap())
            }
        }
    }
}
