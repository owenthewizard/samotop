use crate::{
    common::*,
    io::Io,
    config::{Component, SingleComponent},
};
use std::ops::DerefMut;

use super::TlsCapable;

/// A stream implementing this trait may be able to upgrade to TLS
/// But maybe not...
pub trait MayBeTls: Io {
    fn enable_encryption(&mut self, upgrade: Box<dyn super::Tls>, name: String);
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
    T: Io,
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

    fn enable_encryption(&mut self, upgrade: Box<dyn super::Tls>, name: String) {
        TLSIO::enable_encryption(T::deref_mut(self), upgrade, name)
    }
}

pub trait TlsProviderExt {
    fn upgrade_to_tls_in_place(&self, io: &mut Box<dyn Io>, name: String);
}
impl<T> TlsProviderExt for T
where
    T: TlsProvider + ?Sized,
{
    fn upgrade_to_tls_in_place(&self, io: &mut Box<dyn Io>, name: String) {
        let tlsio = TlsCapable::encrypt_now(
            std::mem::replace(io, Box::new(FallBack)),
            self.get_tls_upgrade(),
            name,
        );
        *io = Box::new(tlsio);
    }
}

pub trait TlsProvider: std::fmt::Debug {
    fn get_tls_upgrade(&self) -> Box<dyn Tls>;
}

pub trait Tls: Sync + Send {
    fn upgrade_to_tls(
        &self,
        stream: Box<dyn Io>,
        name: String,
    ) -> S3Fut<std::io::Result<Box<dyn Io>>>;
}
pub struct TlsService {}
impl Component for TlsService {
    type Target = Arc<dyn Tls>;
}
impl SingleComponent for TlsService {}
