use async_native_tls::TlsAcceptor;
use async_native_tls::TlsConnector;
use samotop_core::io::tls::TlsProvider;
use samotop_core::io::tls::TlsUpgrade;
use samotop_core::{
    common::*,
    io::tls::Io,
    mail::{Builder, MailSetup},
};
use std::fmt;

pub struct NativeTlsProvider<T> {
    inner: Arc<T>,
}

impl<T> Clone for NativeTlsProvider<T> {
    fn clone(&self) -> Self {
        NativeTlsProvider {
            inner: self.inner.clone(),
        }
    }
}

impl From<TlsAcceptor> for NativeTlsProvider<TlsAcceptor> {
    fn from(acceptor: TlsAcceptor) -> Self {
        NativeTlsProvider {
            inner: Arc::new(acceptor),
        }
    }
}
impl From<TlsConnector> for NativeTlsProvider<TlsConnector> {
    fn from(connector: TlsConnector) -> Self {
        NativeTlsProvider {
            inner: Arc::new(connector),
        }
    }
}

impl TlsUpgrade for NativeTlsProvider<TlsAcceptor> {
    fn upgrade_to_tls(
        &self,
        io: Box<dyn Io>,
        _name: String,
    ) -> S3Fut<std::io::Result<Box<dyn Io>>> {
        let acceptor = self.inner.clone();
        let fut = async move {
            match acceptor.accept(io).await {
                Ok(encrypted) => {
                    // match encrypted.peer_certificate() {
                    //     Err(e) => trace!("peer cert error: {:?}", e),
                    //     Ok(None) => trace!("peer cert None."),
                    //     Ok(Some(cert)) => {
                    //         let cert = cert.to_der().unwrap();
                    //         trace!("peer cert present: {:?}", cert.len());
                    //         let mut f = std::fs::File::create("client.crt")?;
                    //         use std::io::Write;
                    //         f.write_all(cert.as_slice())?;
                    //     }
                    // }
                    let encrypted: Box<dyn Io> = Box::new(encrypted);
                    Ok(encrypted)
                }
                Err(e) => Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    format!("Failed to get TLS - {}", e),
                )),
            }
        };
        Box::pin(fut)
    }
}

impl TlsProvider for NativeTlsProvider<TlsAcceptor> {
    fn get_tls_upgrade(&self) -> Option<Box<dyn TlsUpgrade>> {
        Some(Box::new(self.clone()))
    }
}

impl fmt::Debug for NativeTlsProvider<TlsAcceptor> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeTlsProvider<TlsAcceptor>").finish()
    }
}

impl TlsProvider for NativeTlsProvider<TlsConnector> {
    fn get_tls_upgrade(&self) -> Option<Box<dyn TlsUpgrade>> {
        Some(Box::new(NativeTlsProvider::clone(self)))
    }
}

impl TlsUpgrade for NativeTlsProvider<TlsConnector> {
    fn upgrade_to_tls(
        &self,
        stream: Box<dyn Io>,
        name: String,
    ) -> S3Fut<std::io::Result<Box<dyn Io>>> {
        let connector = self.inner.clone();
        Box::pin(async move {
            match connector.connect(name, stream).await {
                Ok(s) => {
                    let s: Box<dyn Io> = Box::new(s);
                    Ok(s)
                }
                Err(e) => Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, e)),
            }
        })
    }
}

impl fmt::Debug for NativeTlsProvider<TlsConnector> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeTlsProvider<TlsAcceptor>").finish()
    }
}

impl MailSetup for NativeTlsProvider<TlsConnector> {
    fn setup(self, builder: &mut Builder) {
        builder.tls = Box::new(self);
    }
}

impl MailSetup for NativeTlsProvider<TlsAcceptor> {
    fn setup(self, builder: &mut Builder) {
        builder.tls = Box::new(self);
    }
}
