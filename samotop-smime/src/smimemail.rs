use crate::SMime;
use log::error;
use samotop_core::{common::*, mail::*, smtp::SmtpSession};
use std::collections::VecDeque;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SMimeMail {
    private_key_file: PathBuf,
    certificate_file: PathBuf,
}

impl SMimeMail {
    pub fn new(private_key_file: PathBuf, certificate_file: PathBuf) -> Self {
        Self {
            private_key_file,
            certificate_file,
        }
    }
}

impl<T: AcceptsDispatch> MailSetup<T> for SMimeMail {
    fn setup(self, config: &mut T) {
        config.add_last_dispatch(self)
    }
}

impl MailDispatch for SMimeMail {
    fn open_mail_body<'a, 's, 'f>(
        &'a self,
        session: &'s mut SmtpSession,
    ) -> S1Fut<'f, DispatchResult>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
            let my_key = match self.private_key_file.to_str() {
                None => {
                    error!("Server private key is not a string");
                    return Err(DispatchError::Temporary);
                }
                Some(key) => key,
            };
            let my_cert = match self.certificate_file.to_str() {
                None => {
                    error!("Server certificate is not a string");
                    return Err(DispatchError::Temporary);
                }
                Some(cert) => cert,
            };
            let mut certs = VecDeque::new();
            for rcpt in session.transaction.rcpts.iter() {
                match rcpt.certificate {
                    None => {
                        error!("Recipient certificate is missing");
                        return Err(DispatchError::Temporary);
                    }
                    Some(Certificate::Bytes(_)) => {
                        unimplemented!("Only files are supported for now")
                    }
                    Some(Certificate::File(ref file)) => certs.push_back(file.as_str()),
                }
            }
            let her_cert = match certs.pop_front() {
                None => {
                    error!("No recipients");
                    return Err(DispatchError::Temporary);
                }
                Some(file) => file,
            };
            let sink = match session.transaction.sink.take() {
                None => {
                    error!("Mail sink is not in transaction");
                    return Err(DispatchError::Temporary);
                }
                Some(sink) => sink,
            };
            session.transaction.sink = Some(Box::pin(
                SMime::sign_and_encrypt(sink, my_key, my_cert, her_cert, certs.into())
                    .expect("todo"),
            ));
            Ok(())
        })
    }
}
