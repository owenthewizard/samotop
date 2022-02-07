use super::{Tls, TlsProvider};
use crate::common::*;
use crate::io::Io;

#[derive(Default, Debug, Clone, Copy)]
pub struct NoTls;

#[derive(Default, Debug, Copy, Clone)]
pub struct Impossible {}

impl TlsProvider for NoTls {
    fn get_tls_upgrade(&self) -> Box<dyn Tls> {
        Box::new(NoTls)
    }
}
impl Tls for NoTls {
    fn upgrade_to_tls(
        &self,
        _stream: Box<dyn Io>,
        _name: String,
    ) -> S3Fut<std::io::Result<Box<dyn Io>>> {
        unreachable!()
        //Ok(Box::pin(ready(stream)))
    }
}
impl Tls for Impossible {
    fn upgrade_to_tls(
        &self,
        _stream: Box<dyn Io>,
        _name: String,
    ) -> S3Fut<std::io::Result<Box<dyn Io>>> {
        unreachable!()
        //Ok(Box::pin(ready(stream)))
    }
}

impl io::Read for Impossible {
    fn poll_read(
        self: Pin<&mut Self>,
        __cx: &mut Context<'_>,
        __buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        unreachable!()
    }
}

impl io::Write for Impossible {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        unreachable!()
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        unreachable!()
    }
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        unreachable!()
    }
}
