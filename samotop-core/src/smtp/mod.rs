pub mod command;
mod driver;
mod esmtp;
pub mod extension;
mod extensions;
mod host;
mod impatience;
mod interpretter;
mod parser;
mod path;
mod prudence;
mod reply;
mod rfc2033;
mod rfc3207;
mod rfc5321;
mod rfc821;
mod session;
mod state;
mod transaction;

pub use self::driver::*;
pub use self::esmtp::*;
pub use self::extensions::*;
pub use self::host::*;
pub use self::impatience::*;
pub use self::interpretter::*;
pub use self::parser::*;
pub use self::path::*;
pub use self::prudence::*;
pub use self::reply::*;
pub use self::rfc2033::*;
pub use self::rfc3207::*;
pub use self::rfc5321::*;
pub use self::rfc5321::*;
pub use self::rfc821::*;
pub use self::session::*;
pub use self::state::*;
pub use self::transaction::*;

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
