use crate::{common::*, io::ConnectionInfo};
use std::ops::{Deref, DerefMut};

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
pub trait IoService<IO> {
    fn handle(&self, io: Result<IO>, connection: ConnectionInfo) -> S3Fut<Result<()>>;
}

impl<IO, S: IoService<IO> + ?Sized, T: Deref<Target = S>> IoService<IO> for T
where
    IO: Sync + Send,
    S: Sync + Send,
    T: Sync + Send,
{
    fn handle(&self, io: Result<IO>, connection: ConnectionInfo) -> S3Fut<Result<()>> {
        S::handle(self.deref(), io, connection)
    }
}

/// A stream implementing this trait may be able to upgrade to TLS
/// But maybe not...
pub trait MayBeTls {
    /// Initiates the TLS negotiations.
    /// The stream must then block all reads/writes until the
    /// underlying TLS handshake is done.
    fn encrypt(self: Pin<&mut Self>) -> std::io::Result<()>;
    /// Returns true only if calling encrypt would make sense:
    /// 1. required encryption setup information is available.
    /// 2. the stream is not encrypted yet.
    fn can_encrypt(&self) -> bool;
    /// Returns true if the stream is already encrypted.
    fn is_encrypted(&self) -> bool;
}

impl<T, TLSIO> MayBeTls for T
where
    T: DerefMut<Target = TLSIO> + Unpin,
    TLSIO: MayBeTls + Unpin,
{
    fn encrypt(mut self: Pin<&mut Self>) -> std::io::Result<()> {
        Pin::new(self.deref_mut()).encrypt()
    }
    fn can_encrypt(&self) -> bool {
        Deref::deref(self).can_encrypt()
    }
    fn is_encrypted(&self) -> bool {
        Deref::deref(self).is_encrypted()
    }
}
