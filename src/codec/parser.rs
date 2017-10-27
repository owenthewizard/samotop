/*
    Aim: wrap generated parser fns in struct and abstract with a trait for DI
*/
pub use super::grammar::{ParseResult, ParseError};
use model::request::SmtpInput;
use super::grammar::session;

static PARSER: SmtpParser = SmtpParser;

pub trait SmtpSessionParser {
    fn session<'a>(&self, input: &'a str) -> ParseResult<Vec<SmtpInput>>;
}

pub struct SmtpParser;

impl SmtpParser {
    pub fn session_parser<'a>() -> &'a SmtpSessionParser {
        &PARSER
    }
}

impl SmtpSessionParser for SmtpParser {
    fn session<'a>(&self, input: &'a str) -> ParseResult<Vec<SmtpInput>> {
        session(input)
    }
}
