mod smtp;

pub use self::smtp::*;
pub use samotop_core::service::tcp::dummy::*;
pub use samotop_core::service::tcp::TcpService;

pub mod tls {

    pub use samotop_core::service::tcp::tls::*;

    #[cfg(feature = "rust-tls")]
    mod tls_impl_rust;

    #[cfg(feature = "rust-tls")]
    pub use tls_impl_rust::*;

    #[cfg(feature = "native-tls")]
    mod tls_impl_native;
}
