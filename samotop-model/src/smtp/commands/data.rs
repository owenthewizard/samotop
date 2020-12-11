use crate::smtp::{SmtpSessionCommand, SmtpState};
use crate::{common::*, mail::DispatchError};

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct SmtpData;
