/*
    Aim: wrap generated parser fns in struct
*/
use super::smtp::grammar::{command, session};
use crate::model::io::ReadControl;
use crate::model::smtp::SmtpCommand;
use peg;

static PARSER: SmtpParser = SmtpParser;

pub trait Parser {
    fn command(&self, input: &[u8]) -> Result<SmtpCommand, peg::error::ParseError<usize>>;
    fn script(&self, input: &[u8]) -> Result<Vec<ReadControl>, peg::error::ParseError<usize>>;
}

#[derive(Clone)]
pub struct SmtpParser;

impl SmtpParser {
    pub fn new() -> SmtpParser {
        PARSER.clone()
    }
}

impl Parser for SmtpParser {
    fn command(&self, input: &[u8]) -> Result<SmtpCommand, peg::error::ParseError<usize>> {
        command(input)
    }
    fn script(&self, input: &[u8]) -> Result<Vec<ReadControl>, peg::error::ParseError<usize>> {
        session(input)
    }
}
