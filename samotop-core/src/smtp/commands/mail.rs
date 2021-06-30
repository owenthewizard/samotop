use crate::smtp::SmtpPath;

/// Starts new mail transaction
#[derive(Eq, PartialEq, Debug, Clone)]
pub enum SmtpMail {
    Mail(SmtpPath, Vec<String>),
    Send(SmtpPath, Vec<String>),
    Saml(SmtpPath, Vec<String>),
    Soml(SmtpPath, Vec<String>),
}

impl SmtpMail {
    pub fn verb(&self) -> &str {
        match self {
            SmtpMail::Mail(_, _) => "MAIL",
            SmtpMail::Send(_, _) => "SEND",
            SmtpMail::Saml(_, _) => "SAML",
            SmtpMail::Soml(_, _) => "SOML",
        }
    }
    pub fn sender(&self) -> &SmtpPath {
        match self {
            SmtpMail::Mail(p, _) => p,
            SmtpMail::Send(p, _) => p,
            SmtpMail::Saml(p, _) => p,
            SmtpMail::Soml(p, _) => p,
        }
    }
}
