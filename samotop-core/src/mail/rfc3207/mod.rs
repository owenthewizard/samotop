mod starttls;

/// An implementation of ESMTP STARTTLS - RFC 3207 - SMTP Service Extension for Secure SMTP over Transport Layer Security
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct ESMTPStartTls;

pub type Rfc3207 = ESMTPStartTls;

impl Rfc3207 {
    pub fn command() -> ESMTPStartTls {
        ESMTPStartTls
    }
}
