use log::error;
use samotop_core::{
    common::{ready, S2Fut},
    mail::{
        AddRecipientFailure, AddRecipientRequest, AddRecipientResult, Certificate, Configuration,
        MailGuard, MailSetup, SessionInfo, StartMailRequest, StartMailResult,
    },
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

impl MailSetup for Accounts {
    fn setup(self, config: &mut Configuration) {
        config.guard.insert(0, Box::new(self))
    }
}

impl MailGuard for Accounts {
    fn add_recipient<'a, 'f>(
        &'a self,
        mut request: AddRecipientRequest,
    ) -> S2Fut<'f, AddRecipientResult>
    where
        'a: 'f,
    {
        if request.rcpt.certificate.is_some() {
            return Box::pin(ready(AddRecipientResult::Inconclusive(request)));
        }
        let mut path = async_std::path::PathBuf::from(&self.accounts_dir);
        // TODO: hash the value for privacy
        path.push(request.rcpt.address.address().to_lowercase());
        path.push("certificate");

        Box::pin(async move {
            if path.exists().await {
                match path.to_str() {
                    Some(cert) => {
                        request.rcpt.certificate = Some(Certificate::File(cert.to_owned()));
                        AddRecipientResult::Inconclusive(request)
                    }
                    None => {
                        error!("Invalid recipient cert path {:?}", path);
                        AddRecipientResult::Failed(
                            request.transaction,
                            AddRecipientFailure::FailedTemporarily,
                            "Not ready".to_owned(),
                        )
                    }
                }
            } else {
                error!("Recipient cert missing {:?}", path);
                AddRecipientResult::Failed(
                    request.transaction,
                    AddRecipientFailure::FailedTemporarily,
                    "Not ready".to_owned(),
                )
            }
        })
    }

    fn start_mail<'a, 's, 'f>(
        &'a self,
        _session: &'s SessionInfo,
        request: StartMailRequest,
    ) -> S2Fut<'f, StartMailResult>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(ready(StartMailResult::Accepted(request)))
    }
}
