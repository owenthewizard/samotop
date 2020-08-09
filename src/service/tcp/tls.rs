use crate::common::*;
use crate::model::io::Connection;
use crate::model::smtp::SmtpExtension;
use crate::protocol::TlsCapable;
use crate::service::tcp::TcpService;
use async_tls::TlsAcceptor;

#[doc = "Dummy TCP service for testing samotop server"]
#[derive(Clone)]
pub struct TlsEnabled<T> {
    acceptor: Option<TlsAcceptor>,
    wrapped: T,
}

impl<T> TlsEnabled<T> {
    pub fn yes(wrapped: T, acceptor: TlsAcceptor) -> Self {
        TlsEnabled::new(wrapped, acceptor)
    }
    pub fn no(wrapped: T) -> Self {
        TlsEnabled::new(wrapped, None)
    }
    pub fn new(wrapped: T, acceptor: impl Into<Option<TlsAcceptor>>) -> Self {
        TlsEnabled {
            wrapped,
            acceptor: acceptor.into(),
        }
    }
}

impl<T, IO> TcpService<IO> for TlsEnabled<T>
where
    T: TcpService<TlsCapable<IO>>,
    IO: Read + Write + Unpin,
{
    type Future = T::Future;
    fn handle(self, io: Result<IO>, mut conn: Connection) -> Self::Future {
        if self.acceptor.is_some() {
            conn.enable(SmtpExtension::StartTls);
        }
        let TlsEnabled { acceptor, wrapped } = self;
        let tls = io.map(|io| TlsCapable::new(io, acceptor));
        wrapped.handle(tls, conn)
    }
}
