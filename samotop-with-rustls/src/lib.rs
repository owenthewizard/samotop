use async_tls::{TlsAcceptor, TlsConnector};
use samotop_model::io::tls::Io;
use samotop_model::io::tls::TlsProvider;
use samotop_model::io::tls::TlsUpgrade;
use samotop_model::{common::*, mail::MailSetup};
use std::fmt;

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
    fn get(&self) -> Option<Box<dyn TlsUpgrade>> {
        Some(Box::new(self.clone()))
    }
}

impl TlsProvider for RustlsProvider<TlsConnector> {
    fn get(&self) -> Option<Box<dyn TlsUpgrade>> {
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

impl MailSetup for RustlsProvider<TlsConnector> {
    fn setup(self, builder: &mut samotop_model::mail::Builder) {
        builder.tls = Box::new(self);
    }
}

impl MailSetup for RustlsProvider<TlsAcceptor> {
    fn setup(self, builder: &mut samotop_model::mail::Builder) {
        builder.tls = Box::new(self);
    }
}
