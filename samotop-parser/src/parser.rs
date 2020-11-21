/*
    Aim: wrap generated parser fns in struct
*/
use crate::grammar::*;
use samotop_model::{
    common::Result,
    parser::Parser,
    smtp::{ReadControl, SmtpCommand, SmtpPath},
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
    fn command(&self, input: &[u8]) -> Result<SmtpCommand> {
        Ok(command(input)?)
    }
    fn script(&self, input: &[u8]) -> Result<Vec<ReadControl>> {
        Ok(session(input)?)
    }
    fn forward_path(&self, input: &[u8]) -> Result<SmtpPath> {
        Ok(path_forward(input)?)
    }
}
