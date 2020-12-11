use crate::smtp::SmtpHost;

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct SmtpHelo {
    pub verb: String,
    pub host: SmtpHost,
}
