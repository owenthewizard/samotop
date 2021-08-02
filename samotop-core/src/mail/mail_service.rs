use crate::{mail::*, smtp::SessionService};

pub trait MailService: SessionService + MailGuard + MailDispatch {}
impl<T> MailService for T where T: SessionService + MailGuard + MailDispatch {}
