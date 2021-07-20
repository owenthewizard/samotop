use async_tls::{TlsAcceptor, TlsConnector};
use samotop_core::{
    common::*,
    io::tls::{Io, TlsProvider, TlsUpgrade},
};
use std::fmt;

/// TLS provider for RustTLS.
///
/// Use the ::from impls with either a TlsAcceptor or TlsConnector
#[derive(Clone)]
pub struct RustlsProvider<T> {
    inner: T,
}

impl From<TlsAcceptor> for RustlsProvider<TlsAcceptor> {
    fn from(acceptor: TlsAcceptor) -> Self {
        RustlsProvider { inner: acceptor }
    }
}
impl From<TlsConnector> for RustlsProvider<TlsConnector> {
    fn from(connector: TlsConnector) -> Self {
        RustlsProvider { inner: connector }
    }
}

impl TlsUpgrade for RustlsProvider<TlsAcceptor> {
    fn upgrade_to_tls(
        &self,
        io: Box<dyn Io>,
        _name: String,
    ) -> S3Fut<std::io::Result<Box<dyn Io>>> {
        let fut = self.inner.accept(io);
        Box::pin(async move {
            match fut.await {
                Ok(encrypted) => {
                    let encrypted: Box<dyn Io> = Box::new(encrypted);
                    Ok(encrypted)
                }
                Err(e) => Err(e),
            }
        })
    }
}

impl TlsUpgrade for RustlsProvider<TlsConnector> {
    fn upgrade_to_tls(&self, io: Box<dyn Io>, name: String) -> S3Fut<std::io::Result<Box<dyn Io>>> {
        let fut = self.inner.connect(name.as_str(), io);
        Box::pin(async move {
            match fut.await {
                Ok(encrypted) => {
                    let encrypted: Box<dyn Io> = Box::new(encrypted);
                    Ok(encrypted)
                }
                Err(e) => Err(e),
            }
        })
    }
}

impl TlsProvider for RustlsProvider<TlsAcceptor> {
    fn get_tls_upgrade(&self) -> Option<Box<dyn TlsUpgrade>> {
        Some(Box::new(self.clone()))
    }
}

impl TlsProvider for RustlsProvider<TlsConnector> {
    fn get_tls_upgrade(&self) -> Option<Box<dyn TlsUpgrade>> {
        Some(Box::new(self.clone()))
    }
}

impl fmt::Debug for RustlsProvider<TlsAcceptor> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RustlsProvider<TlsAcceptor>").finish()
    }
}

impl fmt::Debug for RustlsProvider<TlsConnector> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RustlsProvider<TlsConnector>").finish()
    }
}
