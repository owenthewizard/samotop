/*
    Aim: wrap generated parser fns in struct
*/
use super::smtp::{command, session};
pub use super::smtp::{ParseError, ParseResult};
use model::command::{SmtpCommand, SmtpInput};

static PARSER: SmtpParser = SmtpParser;

#[derive(Clone)]
pub struct SmtpParser;

impl SmtpParser {
    pub fn new() -> SmtpParser {
        PARSER.clone()
    }
    pub fn session<'input>(&self, input: &'input str) -> ParseResult<Vec<SmtpInput>> {
        session(input)
    }
    pub fn command<'input>(&self, input: &'input str) -> ParseResult<SmtpCommand> {
        command(input)
    }
}
