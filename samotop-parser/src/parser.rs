/*
    Aim: wrap generated parser fns in struct
*/
use crate::grammar::*;
use samotop_model::{
    parser::{ParseError, ParseResult, Parser},
    smtp::{SmtpCommand, SmtpPath},
    Error,
};

static PARSER: SmtpParser = SmtpParser;

#[derive(Clone, Debug)]
pub struct SmtpParser;

impl Default for SmtpParser {
    fn default() -> SmtpParser {
        PARSER.clone()
    }
}

impl Parser for SmtpParser {
    fn command<'i>(&self, input: &'i [u8]) -> ParseResult<'i, SmtpCommand> {
        Self::map(command(input), input)
    }
}

impl SmtpParser {
    pub fn forward_path<'i>(&self, input: &'i [u8]) -> ParseResult<'i, SmtpPath> {
        Self::map(path_forward(input), input)
    }
    fn map<'i, T, E>(myres: std::result::Result<T, E>, input: &'i [u8]) -> ParseResult<'i, T>
    where
        E: Into<Error>,
    {
        match myres {
            Ok(item) => Ok((input, item)),
            Err(e) => Err(ParseError::Mismatch(e.into())),
        }
    }
}
