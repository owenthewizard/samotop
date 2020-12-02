/*
    Aim: wrap generated parser fns in struct
*/
use crate::{data::DataParser, smtp::grammar::*};
use samotop_model::{
    common::Arc,
    mail::MailSetup,
    parser::{ParseError, ParseResult, Parser},
    smtp::SmtpPath,
    smtp::SmtpSessionCommand,
    Error,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct SmtpParserPeg;

impl Parser for SmtpParserPeg {
    fn parse_command<'i>(&self, input: &'i [u8]) -> ParseResult<'i, Box<dyn SmtpSessionCommand>> {
        if input.is_empty() {
            return Err(ParseError::Incomplete);
        }
        match command(input) {
            Ok(Ok((input, cmd))) => Ok((input, Box::new(cmd))),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(ParseError::Failed(e.into())),
        }
    }
}

impl MailSetup for SmtpParserPeg {
    fn setup(self, builder: &mut samotop_model::mail::Builder) {
        builder.command_parser.insert(0, Arc::new(self));
        builder.data_parser.insert(0, Arc::new(DataParser));
    }
}

impl SmtpParserPeg {
    pub fn forward_path<'i>(&self, input: &'i [u8]) -> ParseResult<'i, SmtpPath> {
        Self::map(path_forward(input), b"")
    }
    fn map<T, E>(myres: std::result::Result<T, E>, input: &[u8]) -> ParseResult<T>
    where
        E: Into<Error>,
    {
        match myres {
            Ok(item) => Ok((input, item)),
            Err(e) => Err(ParseError::Mismatch(e.into())),
        }
    }
}
