use crate::{
    io::{tls::TlsProvider, IoService},
    mail::*,
};

pub trait MailService:
    IoService + TlsProvider + DriverProvider + EsmtpService + MailGuard + MailDispatch
{
}
impl<T> MailService for T where
    T: IoService + TlsProvider + DriverProvider + EsmtpService + MailGuard + MailDispatch
{
}
