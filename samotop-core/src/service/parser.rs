use crate::common::Result;
use crate::model::io::ReadControl;
use crate::model::smtp::SmtpCommand;

pub trait Parser {
    fn command(&self, input: &[u8]) -> Result<SmtpCommand>;
    fn script(&self, input: &[u8]) -> Result<Vec<ReadControl>>;
}
