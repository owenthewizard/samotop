use crate::DataParserPeg;
use samotop_core::{
    common::*,
    mail::{Builder, MailSetup, Rfc2033},
    parser::{ParseError, ParseResult, Parser},
    smtp::*,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct LmtpParserPeg;

impl MailSetup for LmtpParserPeg {
    fn setup(self, builder: &mut Builder) {
        builder.command_parser.insert(0, Arc::new(self));
        builder
            .data_parser
            .insert(0, Arc::new(DataParserPeg { lmtp: true }));
    }
}

impl Parser for LmtpParserPeg {
    fn parse_command<'i>(&self, input: &'i [u8]) -> ParseResult<'i, Box<dyn SmtpSessionCommand>> {
        if input.is_empty() {
            return Err(ParseError::Incomplete);
        }
        match crate::smtp::grammar::command(input) {
            Err(e) => Err(ParseError::Failed(e.into())),
            Ok(Err(e)) => Err(e),
            Ok(Ok((i, cmd))) => Ok((i, Box::new(Rfc2033::new(cmd)))),
        }
    }
}
