use crate::{
    common::*,
    smtp::{SmtpSessionCommand, SmtpUnknownCommand},
};
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

impl Parser for Vec<Arc<dyn Parser + Sync + Send>> {
    fn parse_command<'i>(&self, input: &'i [u8]) -> ParseResult<'i, Box<dyn SmtpSessionCommand>> {
        for (idx, parser) in self.iter().enumerate() {
            trace!("Parser {} parse_command calling {:?}", idx, parser);
            match parser.parse_command(input) {
                Err(ParseError::Mismatch(e)) => {
                    debug!(
                        "Parser {} - {:?} did not recognize the input: {:?}",
                        idx, parser, e
                    );
                }
                otherwise => return otherwise,
            }
        }
        Err(ParseError::Mismatch("No parser can parse this".into()))
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
impl std::error::Error for ParseError {}

#[derive(Default, Copy, Clone, Debug)]
pub struct DummyParser;

impl Parser for DummyParser {
    fn parse_command<'i>(&self, input: &'i [u8]) -> ParseResult<'i, Box<dyn SmtpSessionCommand>> {
        if let Some(line) = input.split(|b| *b == b'\n').next() {
            Ok((
                &input[line.len() + 1..],
                Box::new(SmtpUnknownCommand::default()),
            ))
        } else {
            Err(ParseError::Incomplete)
        }
    }
}
