//! Reference implementation of a mail guard
//! converting recipient addresses according to a regex map.
use crate::common::*;
use crate::model::mail::*;
use crate::service::mail::composite::*;
use crate::service::mail::*;
use crate::service::parser::Parser;
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
pub struct Mapper<S> {
    config: Config,
    inner: S,
}

impl<S> Mapper<S> {
    pub fn new(config: Config, inner: S) -> Self {
        Self { config, inner }
    }
}

impl<ES, GS, DS> MailSetup<ES, GS, DS> for Config
where
    ES: EsmtpService,
    GS: MailGuard,
    DS: MailDispatch,
{
    type Output = CompositeMailService<ES, Mapper<GS>, DS>;
    fn setup(self, extend: ES, guard: GS, dispatch: DS) -> Self::Output {
        (extend, Mapper::new(self, guard), dispatch).into()
    }
}

impl<S> MailGuard for Mapper<S>
where
    S: MailGuard,
{
    type RecipientFuture =
        Pin<Box<dyn Future<Output = AddRecipientResult> + Sync + Send + 'static>>;

    type SenderFuture = future::Ready<StartMailResult>;

    fn add_recipient(&self, mut request: AddRecipientRequest) -> Self::RecipientFuture {
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
                let fut = self.inner.add_recipient(request);
                Box::pin(fut)
            }
            Err(e) => {
                let err = format!(
                    "Map conversions of {:?} produced invalid forward path {:?}. Error: {}",
                    request.rcpt.to_string(),
                    rcpt,
                    e
                );
                let fut = future::ready(AddRecipientResult::Failed(
                    request.transaction,
                    AddRecipientFailure::FailedTemporarily,
                    err,
                ));
                Box::pin(fut)
            }
        }
    }

    fn start_mail(&self, request: StartMailRequest) -> Self::SenderFuture {
        future::ready(StartMailResult::Accepted(request))
    }
}

#[cfg(test)]
mod tests {
    use crate::model::smtp::SmtpAddress;
    use crate::model::smtp::SmtpHost;
    use crate::model::smtp::SmtpPath;
    use crate::service::mail::default::DefaultMailService;
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
        let sut = Mapper::new(cfg, DefaultMailService::default());
        let req = AddRecipientRequest {
            transaction: Transaction::default(),
            rcpt: SmtpPath::Direct(SmtpAddress::Mailbox(
                "user".to_owned(),
                SmtpHost::Domain("example.org".to_owned()),
            )),
        };

        let res = sut.add_recipient(req).await;
        match res {
            AddRecipientResult::Accepted(t) => {
                assert_eq!(t.rcpts[0].address(), "example-org@localhost")
            }
            other => panic!("Unexpected {:?}", other),
        }
        Ok(())
    }
}
