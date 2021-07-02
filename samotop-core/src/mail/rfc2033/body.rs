use super::Lmtp;
use crate::{
    common::*,
    mail::apply_mail_body,
    smtp::{command::MailBody, Action, SmtpState},
};

impl<B: AsRef<[u8]> + Sync + Send + fmt::Debug + 'static> Action<MailBody<B>> for Lmtp {
    fn apply<'a, 's, 'f>(&'a self, cmd: MailBody<B>, state: &'s mut SmtpState) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(apply_mail_body(true, cmd, state))
    }
}
