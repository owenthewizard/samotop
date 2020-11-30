use super::{TlsProvider, TlsUpgrade};
use crate::common::*;
use crate::io::tls::Io;

#[derive(Default, Debug, Clone, Copy)]
pub struct NoTls;

#[derive(Default, Debug, Copy, Clone)]
pub struct Impossible {}

impl TlsProvider for NoTls {
    type Upgrade = Impossible;
    fn get(&self) -> Option<Self::Upgrade> {
        None
    }
}

impl TlsUpgrade for Impossible {
    fn upgrade_to_tls(
        &self,
        stream: Box<dyn Io>,
        _name: String,
    ) -> S3Fut<std::io::Result<Box<dyn Io>>> {
        unreachable!()
        //Ok(Box::pin(ready(stream)))
    }
}

impl Read for Impossible {
    fn poll_read(
        self: Pin<&mut Self>,
        __cx: &mut Context<'_>,
        __buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        unreachable!()
    }
}

impl Write for Impossible {
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
