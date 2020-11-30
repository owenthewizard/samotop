use super::tls::MayBeTls;
use crate::{common::*, io::ConnectionInfo};
use std::ops::Deref;

/**
An object implementing this trait handles TCP connections in a `Future`.

The caller would ask the service to `handle()` the anaccepted io stream and connection
(most likely a `TcpStream`, but abstracted for testability),
then poll the returned future to completion. The service may chose
to block on the call, effectively preventing concurrent connections,
or better to spawn into background and return immediately so that other
connections can be received. It could also push back on accepting
more connections if it notices too many connections or another resource
shortage by blocking on the `handle()` call.

The `SmtpService` and `DummyService` implement this trait.
*/
pub trait IoService {
    fn handle(
        &self,
        io: Result<Box<dyn MayBeTls>>,
        connection: ConnectionInfo,
    ) -> S3Fut<Result<()>>;
}

impl<S: IoService + ?Sized, T: Deref<Target = S>> IoService for T
where
    S: Sync + Send,
    T: Sync + Send,
{
    fn handle(
        &self,
        io: Result<Box<dyn MayBeTls>>,
        connection: ConnectionInfo,
    ) -> S3Fut<Result<()>> {
        S::handle(self.deref(), io, connection)
    }
}
