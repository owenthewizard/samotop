pub use samotop_core::io::*;

#[cfg(feature = "delivery")]
pub use samotop_delivery::smtp::net as client;

pub mod tls {
    pub use samotop_core::io::tls::*;

    #[cfg(feature = "rust-tls")]
    pub use samotop_with_rustls::*;

    #[cfg(feature = "native-tls")]
    pub use samotop_with_native_tls::*;
}
