//! Reference implementation of a mail guard
//! converting recipient addresses according to a regex map.

use crate::mail::*;
use crate::{common::*, smtp::SmtpParser};
use regex::Regex;

/// A mail guard that converts recipient addresses according to a regex map.
#[derive(Clone, Debug, Default)]
pub struct Mapper {
    map: Vec<(Regex, String)>,
}

impl Mapper {
    pub fn new(map: Vec<(Regex, String)>) -> Self {
        Self { map }
    }
}

impl MailSetup for Mapper {
    fn setup(self, config: &mut Configuration) {
        config.guard.insert(0, Box::new(self))
    }
}

impl MailGuard for Mapper {
    fn add_recipient<'a, 'f>(
        &'a self,
        mut request: AddRecipientRequest,
    ) -> S2Fut<'f, AddRecipientResult>
    where
        'a: 'f,
    {
        let mut rcpt = request.rcpt.address.address();
        for conversion in self.map.iter() {
            rcpt = conversion
                .0
                .replace(rcpt.as_ref(), conversion.1.as_str())
                .into();
        }
        let rcpt = format!("<{}>", rcpt);

        match SmtpParser.forward_path(rcpt.as_bytes()) {
            Ok((i, new_path)) => {
                trace!("Converted {} into {}", request.rcpt.address, rcpt);
                assert_eq!(i, rcpt.len());
                request.rcpt.address = new_path;
                Box::pin(ready(AddRecipientResult::Inconclusive(request)))
            }
            Err(e) => {
                let err = format!(
                    "Map conversions of {:?} produced invalid forward path {:?}. Error: {}",
                    request.rcpt.address.to_string(),
                    rcpt,
                    e
                );
                Box::pin(ready(AddRecipientResult::Failed(
                    request.transaction,
                    AddRecipientFailure::FailedTemporarily,
                    err,
                )))
            }
        }
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

#[cfg(test)]
mod tests {
    use crate::smtp::SmtpHost;
    use crate::smtp::SmtpPath;
    use regex::Regex;

    use super::*;

    #[async_std::test]
    async fn test() -> Result<()> {
        // use the domain as a user, converting to linux like user name
        let sut = Mapper::new(vec![
            (Regex::new(".*@(.*)")?, "$1@localhost".to_owned()),
            (Regex::new("[^@a-zA-Z0-9]")?, "-".to_owned()),
        ]);
        let req = AddRecipientRequest {
            transaction: Transaction::default(),
            rcpt: Recipient::new(SmtpPath::Mailbox {
                name: "user".to_owned(),
                host: SmtpHost::Domain("example.org".to_owned()),
                relays: vec![],
            }),
        };

        let res = sut.add_recipient(req).await;
        match res {
            AddRecipientResult::Inconclusive(request) => {
                assert_eq!(request.rcpt.address.address(), "example-org@localhost")
            }
            other => panic!("Unexpected {:?}", other),
        }
        Ok(())
    }
}
