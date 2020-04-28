/*
    Aim: wrap generated parser fns in struct
*/
use super::smtp::grammar::{command, session};
use crate::model::command::{SmtpCommand, SmtpInput};
use peg;

static PARSER: SmtpParser = SmtpParser;

pub trait Parser {
    fn command<'input>(&self, input: &'input str) -> Result<SmtpCommand, peg::error::ParseError<peg::str::LineCol>>;
}

#[derive(Clone)]
pub struct SmtpParser;

impl SmtpParser {
    pub fn new() -> SmtpParser {
        PARSER.clone()
    }
    pub fn session<'input>(&self, input: &'input str) -> Result<Vec<SmtpInput>, peg::error::ParseError<peg::str::LineCol>> {
        session(input)
    }
    pub fn command<'input>(&self, input: &'input str) -> Result<SmtpCommand, peg::error::ParseError<peg::str::LineCol>> {
        command(input)
    }
}

impl Parser for SmtpParser {
    fn command<'input>(&self, input: &'input str) -> Result<SmtpCommand, peg::error::ParseError<peg::str::LineCol>> {
        self.command(input)
    }
}
