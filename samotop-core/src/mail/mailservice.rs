use crate::{
    mail::*,
    smtp::{EsmtpService, Interpret},
};

pub trait MailService:
    Interpret + DriverProvider + EsmtpService + MailGuard + MailDispatch
{
}
impl<T> MailService for T where
    T: Interpret + DriverProvider + EsmtpService + MailGuard + MailDispatch
{
}
