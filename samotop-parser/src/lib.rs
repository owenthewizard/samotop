#[macro_use]
extern crate log;
mod data;
mod smtp;
pub use self::data::*;
pub use self::smtp::*;
pub use samotop_core::smtp::{ParseError, ParseResult, Parser};

#[derive(Clone, Copy, Debug, Default)]
pub struct SmtpParserPeg;
