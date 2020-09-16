/*
    Aim: wrap generated parser fns in struct
*/
use super::smtp::grammar::{command, session};
use samotop_core::common::Result;
use samotop_core::model::smtp::ReadControl;
use samotop_core::model::smtp::SmtpCommand;
use samotop_core::service::parser::Parser;

static PARSER: SmtpParser = SmtpParser;

#[derive(Clone)]
pub struct SmtpParser;

impl SmtpParser {
    pub fn new() -> SmtpParser {
        PARSER.clone()
    }
}

impl Parser for SmtpParser {
    fn command(&self, input: &[u8]) -> Result<SmtpCommand> {
        Ok(command(input)?)
    }
    fn script(&self, input: &[u8]) -> Result<Vec<ReadControl>> {
        Ok(session(input)?)
    }
}
