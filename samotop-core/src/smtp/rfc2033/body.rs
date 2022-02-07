use super::Lmtp;
use crate::{
    common::*,
    smtp::{apply_mail_body, command::MailBody, Action, SmtpContext},
};

impl<B: AsRef<[u8]> + Sync + Send + fmt::Debug + 'static> Action<MailBody<B>> for Lmtp {
    fn apply<'a, 's, 'f>(&'a self, cmd: MailBody<B>, state: &'s mut SmtpContext) -> S2Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(apply_mail_body(true, cmd, state))
    }
}
