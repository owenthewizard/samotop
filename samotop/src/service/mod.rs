pub mod mail;

#[derive(Clone)]
pub struct Provider<T>(pub T);

pub mod session {
    pub use samotop_core::service::session::*;
}

pub mod parser {
    pub use samotop_core::service::parser::*;
    pub use samotop_parser::*;
}

pub mod tcp {
    pub use samotop_core::service::tcp::*;

    pub mod tls {

        pub use samotop_core::service::tcp::tls::*;

        #[cfg(feature = "rust-tls")]
        mod tls_impl_rust;

        #[cfg(feature = "rust-tls")]
        pub use tls_impl_rust::*;

        #[cfg(feature = "native-tls")]
        mod tls_impl_native;
    }
}

#[cfg(feature = "lmtp-dispatch")]
pub use samotop_to_lmtp::net as client;