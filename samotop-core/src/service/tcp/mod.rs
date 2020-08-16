pub mod dummy;
pub mod tls;

use crate::model::io::*;
use crate::model::Result;
use futures::prelude::*;

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

The `SmtpService` and `DummyTcpService` implement this trait.
*/
pub trait TcpService<IO> {
    type Future: Future<Output = ()> + Send + Sync;
    fn handle(self, io: Result<IO>, connection: Connection) -> Self::Future;
}
