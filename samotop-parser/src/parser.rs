/*
    Aim: wrap generated parser fns in struct
*/
use super::smtp::grammar::*;
use samotop_core::model::smtp::ReadControl;
use samotop_core::model::smtp::SmtpCommand;
use samotop_core::service::parser::Parser;
use samotop_core::{common::Result, model::smtp::SmtpPath};

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
