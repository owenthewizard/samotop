use crate::{
    io::IoService,
    mail::*,
    smtp::{EsmtpService, Interpret},
};

pub trait MailService:
    IoService + Interpret + DriverProvider + EsmtpService + MailGuard + MailDispatch
{
}
impl<T> MailService for T where
    T: IoService + Interpret + DriverProvider + EsmtpService + MailGuard + MailDispatch
{
}
