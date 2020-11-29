#[cfg(any(feature = "parser-peg", feature = "parser-nom"))]
mod mapper;
#[cfg(any(feature = "parser-peg", feature = "parser-nom"))]
pub use self::mapper::*;

pub use samotop_core::mail::*;
pub use samotop_delivery::delivery::*;
#[cfg(feature = "spf")]
pub use samotop_with_spf as spf;
