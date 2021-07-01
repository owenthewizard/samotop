use super::Lmtp;
use crate::{
    common::*,
    mail::apply_mail_body,
    smtp::{command::MailBody, Action, SmtpState},
};

#[async_trait::async_trait]
impl<B: AsRef<[u8]> + Sync + Send + fmt::Debug + 'static> Action<MailBody<B>> for Lmtp {
    async fn apply(&self, cmd: MailBody<B>, state: &mut SmtpState) {
        apply_mail_body(true, cmd, state).await
    }
}
