use crate::smtp::SmtpPath;

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct SmtpRcpt(pub SmtpPath, pub Vec<String>);
