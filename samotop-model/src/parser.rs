use crate::{common::*, smtp::SmtpSessionCommand};
use std::fmt;

pub type ParseResult<'a, T> = std::result::Result<(&'a [u8], T), ParseError>;

pub trait Parser: fmt::Debug {
    fn parse_command<'i>(&self, input: &'i [u8]) -> ParseResult<'i, Box<dyn SmtpSessionCommand>>;
}

impl<T> Parser for Arc<T>
where
    T: Parser,
{
    fn parse_command<'i>(&self, input: &'i [u8]) -> ParseResult<'i, Box<dyn SmtpSessionCommand>> {
        T::parse_command(self, input)
    }
}

#[derive(Debug)]
pub enum ParseError {
    Incomplete,
    Failed(Error),
    Mismatch(Error),
}
impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Incomplete => write!(f, "The input is not complete"),
            ParseError::Failed(e) => write!(f, "The input is invalid, parsing failed: {}", e),
            ParseError::Mismatch(e) => write!(f, "Parser did not match the input: {}", e),
        }
    }
}
