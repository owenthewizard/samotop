use crate::common::*;
use std::ops::{Deref, DerefMut};

/// A stream implementing this trait may be able to upgrade to TLS
/// But maybe not...
pub trait MayBeTls: Unpin + Read + Write + Sync + Send {
    /// Initiates the TLS negotiations.
    /// The stream must then block all reads/writes until the
    /// underlying TLS handshake is done.
    /// If it is not possible to encrypt and subsequent reads/writes must fail.
    fn encrypt(self: Pin<&mut Self>);
    /// Returns true only if calling encrypt would make sense:
    /// 1. required encryption setup information is available.
    /// 2. the stream is not encrypted yet.
    fn can_encrypt(&self) -> bool;
    /// Returns true if the stream is already encrypted.
    fn is_encrypted(&self) -> bool;
}

impl<TLSIO, T: DerefMut<Target = TLSIO>> MayBeTls for T
where
    T: Unpin + Read + Write + Sync + Send,
    TLSIO: MayBeTls + Unpin + ?Sized,
{
    fn encrypt(mut self: Pin<&mut Self>) {
        Pin::new(self.deref_mut()).encrypt()
    }
    fn can_encrypt(&self) -> bool {
        Deref::deref(self).can_encrypt()
    }
    fn is_encrypted(&self) -> bool {
        Deref::deref(self).is_encrypted()
    }
}

pub trait Io: Read + Write + Sync + Send + Unpin {}
impl<T> Io for T where T: Read + Write + Sync + Send + Unpin {}

pub trait TlsProvider: std::fmt::Debug {
    type Upgrade: TlsUpgrade + Sync + Send;
    fn get(&self) -> Option<Self::Upgrade>;
}

pub trait TlsUpgrade: Sync + Send {
    fn upgrade_to_tls(
        &self,
        stream: Box<dyn Io>,
        name: String,
    ) -> S3Fut<std::io::Result<Box<dyn Io>>>;
}
