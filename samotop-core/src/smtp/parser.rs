use crate::{
    common::Dummy,
    smtp::{command::SmtpInvalidCommand, SmtpContext},
};
use std::{
    fmt::{self, Debug},
    ops::Deref,
};

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
    fn parse(&self, input: &[u8], state: &SmtpContext) -> ParseResult<CMD>;
}

impl<CMD, S: Parser<CMD>, T: Deref<Target = S> + Debug> Parser<CMD> for T {
    fn parse(&self, input: &[u8], state: &SmtpContext) -> ParseResult<CMD> {
        S::parse(Deref::deref(self), input, state)
    }
}

// impl Parser<SmtpUnknownCommand> for Dummy {
//     fn parse(&self, input: &[u8], _state: &SmtpContext) -> ParseResult<SmtpUnknownCommand> {
//         if let Some(line) = input.split(|b| *b == b'\n').next() {
//             Ok((line.len() + 1, Default::default()))
//         } else {
//             Err(ParseError::Incomplete)
//         }
//     }
// }

impl Parser<SmtpInvalidCommand> for Dummy {
    fn parse(&self, input: &[u8], _state: &SmtpContext) -> ParseResult<SmtpInvalidCommand> {
        if let Some(line) = input.split(|b| *b == b'\n').next() {
            Ok((
                line.len() + 1,
                SmtpInvalidCommand::new(line[0..line.len()].to_vec()),
            ))
        } else {
            Err(ParseError::Incomplete)
        }
    }
}
