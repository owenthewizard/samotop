use crate::common::*;
use crate::model::smtp::ReadControl;
use crate::model::smtp::SmtpCommand;

pub trait Parser {
    fn command(&self, input: &[u8]) -> Result<SmtpCommand>;
    fn script(&self, input: &[u8]) -> Result<Vec<ReadControl>>;
}

impl<T> Parser for Arc<T>
where
    T: Parser,
{
    fn command(&self, input: &[u8]) -> Result<SmtpCommand> {
        T::command(self, input)
    }
    fn script(&self, input: &[u8]) -> Result<Vec<ReadControl>> {
        T::script(self, input)
    }
}
