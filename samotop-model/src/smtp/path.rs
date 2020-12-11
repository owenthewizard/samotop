use super::SmtpHost;
use crate::common::*;

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum SmtpPath {
    Mailbox {
        name: String,
        host: SmtpHost,
        relays: Vec<SmtpHost>,
    },
    Postmaster,
    Null,
}

impl SmtpPath {
    pub fn address(&self) -> String {
        match *self {
            SmtpPath::Null => String::new(),
            SmtpPath::Postmaster => "POSTMASTER".to_owned(),
            SmtpPath::Mailbox {
                ref name, ref host, ..
            } => format!("{}@{}", name, host),
        }
    }
}

impl fmt::Display for SmtpPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<{}>", self.address())
    }
}
