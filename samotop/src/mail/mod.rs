#[cfg(all(
    feature = "mapper",
    any(feature = "parser-peg", feature = "parser-nom")
))]
mod mapper;
#[cfg(all(
    feature = "mapper",
    any(feature = "parser-peg", feature = "parser-nom")
))]
pub use self::mapper::*;

pub use samotop_core::mail::*;

#[cfg(feature = "delivery")]
pub use samotop_delivery::prelude::*;

#[cfg(feature = "spf")]
pub use samotop_with_spf as spf;

#[cfg(feature = "smime")]
pub use samotop_smime as smime;
