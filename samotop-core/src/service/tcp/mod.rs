pub mod dummy;
pub mod smtp;
pub mod tls;

use crate::common::*;
use crate::model::io::*;
use crate::model::Result;
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

The `SmtpService` and `DummyTcpService` implement this trait.
*/
#[async_trait]
pub trait TcpService<IO> {
    #[future_is[Send + Sync + 'static]]
    async fn handle(&self, io: Result<IO>, connection: ConnectionInfo) -> Result<()>;
}

#[async_trait]
impl<IO, S: TcpService<IO> + ?Sized, T: Deref<Target = S>> TcpService<IO> for T
where
    IO: Sync + Send,
    S: Sync + Send,
    T: Sync + Send,
{
    #[future_is[Send + Sync + 'static]]
    async fn handle(&self, io: Result<IO>, connection: ConnectionInfo) -> Result<()> {
        let fut = S::handle(self.deref(), io, connection);
        async_setup_ready!();
        fut.await
    }
}
