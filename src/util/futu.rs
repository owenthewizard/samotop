use tokio::prelude::*;

pub fn pending<I, E>() -> Poll<Option<I>, E> {
    Ok(Async::NotReady)
}
pub fn ok<I, E>(c: I) -> Poll<Option<I>, E> {
    Ok(Async::Ready(Some(c)))
}
pub fn none<I, E>() -> Poll<Option<I>, E> {
    Ok(Async::Ready(None))
}
