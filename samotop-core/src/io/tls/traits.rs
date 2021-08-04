use crate::common::*;
use std::ops::{Deref, DerefMut};

/// A stream implementing this trait may be able to upgrade to TLS
/// But maybe not...
pub trait MayBeTls: Unpin + io::Read + io::Write + Sync + Send {
    fn enable_encryption(&mut self, upgrade: Box<dyn super::TlsUpgrade>, name: String);
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
    T: Unpin + io::Read + io::Write + Sync + Send,
    TLSIO: MayBeTls + Unpin + ?Sized,
{
    fn encrypt(self: Pin<&mut Self>) {
        TLSIO::encrypt(Pin::new(DerefMut::deref_mut(self.get_mut())))
    }
    fn can_encrypt(&self) -> bool {
        TLSIO::can_encrypt(T::deref(self))
    }
    fn is_encrypted(&self) -> bool {
        TLSIO::is_encrypted(T::deref(self))
    }

    fn enable_encryption(&mut self, upgrade: Box<dyn super::TlsUpgrade>, name: String) {
        TLSIO::enable_encryption(T::deref_mut(self), upgrade, name)
    }
}

pub trait Io: io::Read + io::Write + Sync + Send + Unpin {}
impl<T> Io for T where T: io::Read + io::Write + Sync + Send + Unpin {}

pub trait TlsProvider: std::fmt::Debug {
    fn get_tls_upgrade(&self) -> Option<Box<dyn TlsUpgrade>>;
}

pub trait TlsUpgrade: Sync + Send {
    fn upgrade_to_tls(
        &self,
        stream: Box<dyn Io>,
        name: String,
    ) -> S3Fut<std::io::Result<Box<dyn Io>>>;
}

impl<S: TlsProvider + ?Sized, T: Deref<Target = S>> TlsProvider for T
where
    T: fmt::Debug + Send + Sync,
    S: Sync,
{
    fn get_tls_upgrade(&self) -> Option<Box<dyn crate::io::tls::TlsUpgrade>> {
        S::get_tls_upgrade(Deref::deref(self))
    }
}
