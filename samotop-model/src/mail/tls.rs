use crate::common::*;
use crate::io::tls::TlsProvider;

impl<T> TlsProvider for Arc<T>
where
    T: TlsProvider,
{
    fn get_tls_upgrade(&self) -> Option<Box<dyn crate::io::tls::TlsUpgrade>> {
        T::get_tls_upgrade(self)
    }
}
