mod notls;
mod stream;
mod traits;

use core::panic;

pub use notls::*;
pub use stream::*;
pub use traits::*;

use crate::common::*;

impl Read for Dummy {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        panic!("Cannot read on dummy IO")
    }
}

impl Write for Dummy {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        panic!("Cannot write on dummy IO")
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        panic!("Cannot flush on dummy IO")
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // tolerate close
        Poll::Ready(Ok(()))
    }
}

impl MayBeTls for Dummy {
    fn enable_encryption(&mut self, _upgrade: Box<dyn self::TlsUpgrade>, _name: String) {
        panic!("Cannot enable encryption on dummy IO")
    }

    fn encrypt(self: std::pin::Pin<&mut Self>) {
        panic!("Cannot encryptn on dummy IO")
    }

    fn can_encrypt(&self) -> bool {
        false
    }

    fn is_encrypted(&self) -> bool {
        false
    }
}
