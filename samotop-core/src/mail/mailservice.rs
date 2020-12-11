use crate::{io::tls::TlsProvider, mail::*};

pub trait MailService:
    TlsProvider + ParserProvider + EsmtpService + MailGuard + MailDispatch
{
}
impl<T> MailService for T where
    T: TlsProvider + ParserProvider + EsmtpService + MailGuard + MailDispatch
{
}
