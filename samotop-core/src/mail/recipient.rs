use crate::smtp::SmtpPath;
#[derive(Debug, Clone)]
pub struct Recipient {
    pub address: SmtpPath,
    pub certificate: Option<Certificate>,
}

#[derive(Debug, Clone)]
pub enum Certificate {
    File(String),
    Bytes(Vec<u8>),
}

impl Recipient {
    pub fn null() -> Self {
        Self::new(SmtpPath::Null)
    }
    pub fn new(address: SmtpPath) -> Self {
        Recipient {
            address,
            certificate: None,
        }
    }
}
