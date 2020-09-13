use crate::smtp::*;
use std::fmt::{Debug, Display, Formatter, Result as FmtRes};

pub trait Command: Display {
    type Value: CommandValue;
    fn parse(&self, input: &str) -> Result<Option<Self::Value>, Error>;
}
pub trait CommandValue: Display + Debug + Clone {
    type Command: Command;
    fn extension(&self) -> &Self::Command;
}

#[derive(Eq, PartialEq, Debug, Clone, Copy, Hash)]
pub enum Error {
    Incomplete,
    Invalid(usize),
}
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtRes {
        write!(f, "Parsing SMTP command failed. ")?;
        match self {
            Error::Incomplete => write!(f, "The input is incomplete."),
            Error::Invalid(at) => write!(f, "The input is invalid at {}.", at),
        }
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Copy, Hash)]
pub struct BasicCommand {
    code: &'static str,
}
impl Command for BasicCommand {
    type Value = SmtpCommand;
    fn parse(&self, input: &str) -> Result<Option<Self::Value>, Error> {
        todo!()
    }
}
impl Display for BasicCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtRes {
        f.write_str(self.code)
    }
}
impl CommandValue for SmtpCommand {
    type Command = BasicCommand;
    fn extension(&self) -> &Self::Command {
        todo!()
    }
}
impl Display for SmtpCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtRes {
        todo!()
    }
}
