#[macro_use]
extern crate log;

mod lookup;

use self::lookup::*;
use samotop_core::{
    common::*,
    mail::{AcceptsDispatch, DispatchError, DispatchResult, MailDispatch, MailSetup},
    smtp::{SmtpPath, SmtpSession},
};
pub use viaspf::Config;
use viaspf::{evaluate_spf, SpfResult};

/// enables checking for SPF records
#[derive(Clone, Debug)]
pub struct Spf;

impl Spf {
    /// use viaspf config
    pub fn with_config(self, config: Config) -> SpfWithConfig {
        SpfWithConfig {
            config: Arc::new(config),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SpfWithConfig {
    config: Arc<Config>,
}

impl<T: AcceptsDispatch> MailSetup<T> for SpfWithConfig {
    fn setup(self, config: &mut T) {
        config.add_last_dispatch(self)
    }
}
impl<T: AcceptsDispatch> MailSetup<T> for Spf {
    fn setup(self, config: &mut T) {
        config.add_last_dispatch(Spf.with_config(Config::default()))
    }
}

impl MailDispatch for SpfWithConfig {
    fn open_mail_body<'a, 's, 'f>(
        &'a self,
        session: &'s mut SmtpSession,
    ) -> S1Fut<'f, DispatchResult>
    where
        'a: 'f,
        's: 'f,
    {
        let peer_addr = match session.connection.peer_addr.as_str().parse() {
            Err(_) => std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
            Ok(ip) => ip,
        };
        let peer_name = session.peer_name.clone().unwrap_or_default();
        let sender = match session.transaction.mail.as_ref().map(|m| m.sender()) {
            None | Some(SmtpPath::Null) | Some(SmtpPath::Postmaster) => String::new(),
            Some(SmtpPath::Mailbox { host, .. }) => host.domain(),
        };
        let fut = async move {
            // TODO: improve privacy - a) encrypt DNS, b) do DNS servers need to know who is receiving mail from whom?
            let resolver = match new_resolver().await {
                Err(e) => {
                    error!("Could not crerate resolver! {:?}", e);
                    return Err(DispatchError::Temporary);
                }
                Ok(resolver) => resolver,
            };
            let evaluation = evaluate_spf(
                &resolver,
                &self.config,
                peer_addr,
                sender.as_str(),
                peer_name.as_str(),
            )
            .await;
            match evaluation.result {
                SpfResult::Fail(explanation) => {
                    info!("mail rejected due to SPF fail: {}", explanation);
                    Err(DispatchError::Permanent)
                }
                result => {
                    debug!("mail OK with SPF result: {}", result);
                    session
                        .transaction
                        .extra_headers
                        .push_str(format!("X-Samotop-SPF: {}\r\n", result).as_str());
                    Ok(())
                }
            }
        };

        Box::pin(fut)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mail_fut_is_sync() {
        let mut sess = SmtpSession::default();
        let cfg = Config::default();
        let sut = Spf.with_config(cfg);
        let fut = sut.open_mail_body(&mut sess);
        is_send(fut);
    }

    #[test]
    fn config_is_sync() {
        let cfg = Config::default();
        is_sync(cfg);
    }

    fn is_sync<T: Sync>(_subject: T) {}
    fn is_send<T: Send>(_subject: T) {}
}
