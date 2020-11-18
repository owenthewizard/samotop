use crate::common::*;
use crate::smtp::ReadControl;
use crate::smtp::SmtpCommand;
use samotop_model::smtp::SmtpPath;

pub trait Parser {
    fn command(&self, input: &[u8]) -> Result<SmtpCommand>;
    fn script(&self, input: &[u8]) -> Result<Vec<ReadControl>>;
    fn forward_path(&self, input: &[u8]) -> Result<SmtpPath>;
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
    fn forward_path(&self, input: &[u8]) -> Result<SmtpPath> {
        T::forward_path(self, input)
    }
}
