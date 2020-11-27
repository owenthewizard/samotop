/*
    Aim: wrap generated parser fns in struct
*/
use crate::grammar::*;
use memchr::memchr;
use samotop_model::{
    mail::MailSetup,
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
    fn parse_command<'i>(&self, input: &'i [u8]) -> ParseResult<'i, SmtpCommand> {
        let eol = memchr(b'\n', input)
            .map(|lf| lf + 1)
            .unwrap_or_else(|| input.len());
        let (line, input) = input.split_at(eol);
        trace!(
            "PARSING {}, remains {}. input: {:?}",
            eol,
            input.len(),
            String::from_utf8_lossy(line)
        );
        Self::map(command(line), input)
    }
}

impl MailSetup for SmtpParser {
    fn setup(self, builder: &mut samotop_model::mail::Builder) {
        builder.parser.insert(0, Box::new(self))
    }
}

impl SmtpParser {
    pub fn forward_path<'i>(&self, input: &'i [u8]) -> ParseResult<'i, SmtpPath> {
        Self::map(path_forward(input), b"")
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
