use crate::{smtp::net::tls::*, SyncFuture};
use async_std::io::{Read, Write};

#[derive(Default, Debug, Copy, Clone)]
pub struct NoTls;
#[derive(Default, Debug, Copy, Clone)]
pub struct Impossible {}

impl<IO> TlsProvider<IO> for NoTls
where
    IO: Read + Write + Unpin + Send + Sync + 'static,
{
    type Upgrade = Impossible;
    fn get(&self) -> Option<Self::Upgrade> {
        None
    }
}

impl<IO> TlsUpgrade<IO> for Impossible
where
    IO: Read + Write + Unpin + Send + Sync + 'static,
{
    type Encrypted = IO;
    fn upgrade_to_tls(
        self,
        _stream: IO,
        _name: String,
    ) -> SyncFuture<'static, std::io::Result<Self::Encrypted>> {
        unreachable!("TLS upgrade must not be called on NoTls")
    }
}
