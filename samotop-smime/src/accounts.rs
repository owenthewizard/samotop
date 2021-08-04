use log::error;
use samotop_core::{
    common::{ready, S2Fut},
    mail::{
        AcceptsGuard, AddRecipientFailure, AddRecipientResult, Certificate, MailGuard, MailSetup,
        StartMailResult,
    },
    smtp::SmtpSession,
};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Accounts {
    accounts_dir: PathBuf,
}

impl Accounts {
    pub fn new(accounts_dir: PathBuf) -> Self {
        Self { accounts_dir }
    }
}

impl<T: AcceptsGuard> MailSetup<T> for Accounts {
    fn setup(self, config: &mut T) {
        config.add_last_guard(self)
    }
}

impl MailGuard for Accounts {
    fn add_recipient<'a, 's, 'f>(
        &'a self,
        _session: &'s mut SmtpSession,
        mut rcpt: samotop_core::mail::Recipient,
    ) -> S2Fut<'f, AddRecipientResult>
    where
        'a: 'f,
        's: 'f,
    {
        if rcpt.certificate.is_some() {
            return Box::pin(ready(AddRecipientResult::Inconclusive(rcpt)));
        }
        let mut path = async_std::path::PathBuf::from(&self.accounts_dir);
        // TODO: hash the value for privacy
        path.push(rcpt.address.address().to_lowercase());
        path.push("certificate");

        Box::pin(async move {
            if path.exists().await {
                match path.to_str() {
                    Some(cert) => {
                        rcpt.certificate = Some(Certificate::File(cert.to_owned()));
                        AddRecipientResult::Inconclusive(rcpt)
                    }
                    None => {
                        error!("Invalid recipient cert path {:?}", path);
                        AddRecipientResult::Failed(
                            AddRecipientFailure::FailedTemporarily,
                            "Not ready".to_owned(),
                        )
                    }
                }
            } else {
                error!("Recipient cert missing {:?}", path);
                AddRecipientResult::Failed(
                    AddRecipientFailure::FailedTemporarily,
                    "Not ready".to_owned(),
                )
            }
        })
    }

    fn start_mail<'a, 's, 'f>(&'a self, _session: &'s mut SmtpSession) -> S2Fut<'f, StartMailResult>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(ready(StartMailResult::Accepted))
    }
}
