//! Reference implementation of a mail guard
//! converting recipient addresses according to a regex map.

use crate::{
    common::*,
    mail::*,
    smtp::{SmtpParser, SmtpSession},
};
use log::*;
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

impl<T: AcceptsGuard> MailSetup<T> for Mapper {
    fn setup(self, config: &mut T) {
        config.add_last_guard(self)
    }
}

impl MailGuard for Mapper {
    fn add_recipient<'a, 's, 'f>(
        &'a self,
        _session: &'s mut SmtpSession,
        mut rcpt: Recipient,
    ) -> S2Fut<'f, AddRecipientResult>
    where
        'a: 'f,
        's: 'f,
    {
        let mut addr = rcpt.address.address();
        for conversion in self.map.iter() {
            addr = conversion
                .0
                .replace(addr.as_ref(), conversion.1.as_str())
                .into();
        }
        let addr = format!("<{}>", addr);

        match SmtpParser.forward_path(addr.as_bytes()) {
            Ok((i, new_path)) => {
                trace!("Converted {} into {}", rcpt.address, addr);
                assert_eq!(i, addr.len());
                rcpt.address = new_path;
                Box::pin(ready(AddRecipientResult::Inconclusive(rcpt)))
            }
            Err(e) => {
                let err = format!(
                    "Map conversions of {:?} produced invalid forward path {:?}. Error: {}",
                    rcpt.address.to_string(),
                    addr,
                    e
                );
                Box::pin(ready(AddRecipientResult::Failed(
                    AddRecipientFailure::FailedTemporarily,
                    err,
                )))
            }
        }
    }

    fn start_mail<'a, 's, 'f>(&'a self, _session: &'s mut SmtpSession) -> S2Fut<'f, StartMailResult>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(ready(StartMailResult::Accepted))
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
        let mut sess = SmtpSession::default();
        let rcpt = Recipient::new(SmtpPath::Mailbox {
            name: "user".to_owned(),
            host: SmtpHost::Domain("example.org".to_owned()),
            relays: vec![],
        });

        let res = sut.add_recipient(&mut sess, rcpt).await;
        match res {
            AddRecipientResult::Inconclusive(rcpt) => {
                assert_eq!(rcpt.address.address(), "example-org@localhost")
            }
            other => panic!("Unexpected {:?}", other),
        }
        Ok(())
    }
}
