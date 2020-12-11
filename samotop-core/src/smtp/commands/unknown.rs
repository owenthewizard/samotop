#[derive(Default, Eq, PartialEq, Debug, Clone)]
pub struct SmtpUnknownCommand {
    pub verb: String,
    pub params: Vec<String>,
}

impl SmtpUnknownCommand {
    pub fn new(verb: String, params: Vec<String>) -> Self {
        Self { verb, params }
    }
}
