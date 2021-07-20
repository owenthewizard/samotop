pub mod command;
mod driver;
pub mod extension;
mod extensions;
mod host;
mod impatience;
mod interpretter;
mod parser;
mod path;
mod prudence;
mod reply;
mod state;

pub use self::driver::*;
pub use self::extensions::*;
pub use self::host::*;
pub use self::impatience::*;
pub use self::interpretter::*;
pub use self::parser::*;
pub use self::path::*;
pub use self::prudence::*;
pub use self::reply::*;
pub use self::state::*;

/// Represents the instructions for the client side of the stream.
pub enum DriverControl {
    /// Write an SMTP response
    Response(Vec<u8>),
    /// Start TLS encryption
    StartTls,
    /// Shut the stream down
    Shutdown,
}

impl std::fmt::Debug for DriverControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[derive(Debug)]
        enum TextOrBytes<'a> {
            T(&'a str),
            B(&'a [u8]),
        }
        fn tb(inp: &[u8]) -> TextOrBytes {
            if let Ok(text) = std::str::from_utf8(inp) {
                TextOrBytes::T(text)
            } else {
                TextOrBytes::B(inp)
            }
        }
        match self {
            DriverControl::Response(r) => f.debug_tuple("Response").field(&tb(r)).finish(),
            DriverControl::StartTls => f.debug_tuple("StartTls").finish(),
            DriverControl::Shutdown => f.debug_tuple("Shutdown").finish(),
        }
    }
}
