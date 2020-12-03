use crate::DataParserPeg;
use samotop_model::{
    common::*,
    mail::MailSetup,
    parser::{ParseError, ParseResult, Parser},
    smtp::*,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct LmtpParserPeg;

impl MailSetup for LmtpParserPeg {
    fn setup(self, builder: &mut samotop_model::mail::Builder) {
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
            Ok(Ok((i, cmd))) => Ok((
                i,
                match cmd {
                    SmtpCommand::Helo(helo) => match helo {
                        lhlo @ SmtpHelo::Lhlo(_) => Box::new(lhlo),
                        _ => Box::new(SmtpUnknownCommand::default()),
                    },
                    _ => Box::new(cmd),
                },
            )),
        }
    }
}
