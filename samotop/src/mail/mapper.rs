//! Reference implementation of a mail guard
//! converting recipient addresses according to a regex map.
use crate::common::*;
use crate::mail::*;
use crate::parser::Parser;
use regex::Regex;
use samotop_parser::SmtpParser;

#[derive(Clone, Debug, Default)]
pub struct Config {
    map: Vec<(Regex, String)>,
}

impl Config {
    pub fn new(map: Vec<(Regex, String)>) -> Self {
        Self { map }
    }
}

#[derive(Clone, Debug)]
pub struct Mapper {
    config: Config,
}

impl Mapper {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

impl MailSetup for Config {
    fn setup(self, builder: &mut Builder) {
        builder.guard.push(Box::new(Mapper::new(self)))
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
        let mut rcpt = request.rcpt.address();
        for conversion in self.config.map.iter() {
            rcpt = conversion
                .0
                .replace(rcpt.as_ref(), conversion.1.as_str())
                .into();
        }
        let rcpt = format!("<{}>", rcpt);
        match SmtpParser.forward_path(rcpt.as_bytes()) {
            Ok(new_path) => {
                trace!("Converted {} into {}", request.rcpt, rcpt);
                request.rcpt = new_path;
                Box::pin(ready(AddRecipientResult::Inconclusive(request)))
            }
            Err(e) => {
                let err = format!(
                    "Map conversions of {:?} produced invalid forward path {:?}. Error: {}",
                    request.rcpt.to_string(),
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
    use crate::smtp::SmtpAddress;
    use crate::smtp::SmtpHost;
    use crate::smtp::SmtpPath;
    use futures_await_test::async_test;
    use regex::Regex;

    use super::*;

    #[async_test]
    async fn test() -> Result<()> {
        // use the domain as a user, converting to linux like user name
        let cfg = Config::new(vec![
            (Regex::new(".*@(.*)")?, "$1@localhost".to_owned()),
            (Regex::new("[^@a-zA-Z0-9]")?, "-".to_owned()),
        ]);
        let sut = Mapper::new(cfg);
        let req = AddRecipientRequest {
            transaction: Transaction::default(),
            rcpt: SmtpPath::Direct(SmtpAddress::Mailbox(
                "user".to_owned(),
                SmtpHost::Domain("example.org".to_owned()),
            )),
        };

        let res = sut.add_recipient(req).await;
        match res {
            AddRecipientResult::Inconclusive(request) => {
                assert_eq!(request.rcpt.address(), "example-org@localhost")
            }
            other => panic!("Unexpected {:?}", other),
        }
        Ok(())
    }
}
