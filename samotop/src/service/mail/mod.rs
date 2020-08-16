
pub mod dirmail;

#[cfg(feature = "spf")]
pub mod spf;

pub use samotop_core::service::mail::composite::*;
pub use samotop_core::service::mail::default::*;
pub use samotop_core::service::mail::*;
