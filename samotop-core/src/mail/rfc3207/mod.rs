mod starttls;

/// An implementation of ESMTP STARTTLS - RFC 3207 - SMTP Service Extension for Secure SMTP over Transport Layer Security
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct EsmtpStartTls;

pub type Rfc3207 = EsmtpStartTls;
