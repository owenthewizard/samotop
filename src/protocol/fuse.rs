use model::controll::ClientControll;
use tokio::prelude::*;
use util::*;

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

impl<S> FuseShutdown for S
where
    S: Stream,
{
}

pub struct Fuse<S> {
    stream: S,
    trip: bool,
}

impl<S> Stream for Fuse<S>
where
    S: Stream<Item = ClientControll>,
{
    type Item = ClientControll;
    type Error = S::Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        if self.trip {
            return none();
        }

        match try_ready!(self.stream.poll()) {
            None => none(),
            Some(c @ ClientControll::Shutdown) => {
                self.trip = true;
                ok(c)
            }
            Some(c) => ok(c),
        }
    }
}
