mod codec;
mod command;
mod commands;
mod driver;
pub mod extension;
mod extensions;
mod host;
mod path;
mod reply;
mod state;

pub use self::driver::*;
pub use self::codec::*;
pub use self::command::*;
pub use self::extensions::*;
pub use self::host::*;
pub use self::path::*;
pub use self::reply::*;
pub use self::state::*;
use crate::parser::Parser;
use std::fmt;

/// Represents the instructions for the client side of the stream.
pub enum CodecControl {
    /// Write an SMTP response
    Response(Vec<u8>),
    /// Switch parser
    Parser(Box<dyn Parser + Sync + Send>),
    /// Start TLS encryption
    StartTls,
    /// Shut the stream down
    Shutdown,
}

impl fmt::Debug for CodecControl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
            CodecControl::Parser(p) => f.debug_tuple("Parser").field(&p).finish(),
            CodecControl::Response(r) => f.debug_tuple("Response").field(&tb(r)).finish(),
            CodecControl::StartTls => f.debug_tuple("StartTls").finish(),
            CodecControl::Shutdown => f.debug_tuple("Shutdown").finish(),
        }
    }
}
