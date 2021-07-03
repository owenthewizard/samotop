use crate::smtp::{command::SmtpUnknownCommand, Dummy, SmtpState};
use std::fmt::{self, Debug};

#[derive(Debug)]
pub enum ParseError {
    Incomplete,
    Failed(String),
    Mismatch(String),
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
impl std::error::Error for ParseError {}

pub type ParseResult<T> = std::result::Result<(usize, T), ParseError>;

pub trait Parser<CMD>: fmt::Debug {
    fn parse(&self, input: &[u8], state: &SmtpState) -> ParseResult<CMD>;
}

impl Parser<SmtpUnknownCommand> for Dummy {
    fn parse(&self, input: &[u8], _state: &SmtpState) -> ParseResult<SmtpUnknownCommand> {
        if let Some(line) = input.split(|b| *b == b'\n').next() {
            Ok((line.len(), Default::default()))
        } else {
            Err(ParseError::Incomplete)
        }
    }
}
