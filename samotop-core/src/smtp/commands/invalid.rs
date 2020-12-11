#[derive(Default, Eq, PartialEq, Debug, Clone)]
pub struct SmtpInvalidCommand {
    line: Vec<u8>,
}

impl SmtpInvalidCommand {
    pub fn new(line: Vec<u8>) -> Self {
        Self { line }
    }
}
