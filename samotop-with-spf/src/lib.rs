#[macro_use]
extern crate log;

mod lookup;

use self::lookup::*;
use samotop_core::common::*;
use samotop_core::mail::{AcceptsDispatch, DispatchError, DispatchResult, MailDispatch, MailSetup};
use samotop_core::smtp::{SessionInfo, SmtpPath, Transaction};
pub use viaspf::Config;
use viaspf::{evaluate_spf, SpfResult};

/// MailSetup that adds SPF check. If the SPF check results in Fail, mail is rejected.
pub fn provide_viaspf() -> Provider<Config> {
    Provider(Config::default())
}

#[derive(Clone, Debug, Default)]
pub struct Provider<T>(pub T);

#[derive(Clone, Debug)]
pub struct SpfService {
    config: Arc<Config>,
}

impl SpfService {
    pub fn new(config: Config) -> Self {
        Self {
            config: Arc::new(config),
        }
    }
}

impl<T: AcceptsDispatch> MailSetup<T> for Provider<Config> {
    fn setup(self, config: &mut T) {
        config.add_dispatch(SpfService::new(self.0))
    }
}

impl MailDispatch for SpfService {
    fn send_mail<'a, 's, 'f>(
        &'a self,
        session: &'s SessionInfo,
        mut transaction: Transaction,
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
        let sender = match transaction.mail.as_ref().map(|m| m.sender()) {
            None | Some(SmtpPath::Null) | Some(SmtpPath::Postmaster) => String::new(),
            Some(SmtpPath::Mailbox { host, .. }) => host.domain(),
        };
        let fut = async move {
            // TODO: improve privacy - a) encrypt DNS, b) do DNS servers need to know who is receiving mail from whom?
            let resolver = match new_resolver().await {
                Err(e) => {
                    error!("Could not crerate resolver! {:?}", e);
                    return Err(DispatchError::FailedTemporarily);
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
                    Err(DispatchError::Refused)
                }
                result => {
                    debug!("mail OK with SPF result: {}", result);
                    transaction
                        .extra_headers
                        .push_str(format!("X-Samotop-SPF: {}\r\n", result).as_str());
                    Ok(transaction)
                }
            }
        };

        Box::pin(fut)
    }
}

#[cfg(test)]
mod tests {
    use samotop_core::io::ConnectionInfo;

    use super::*;

    #[test]
    fn default_mail_fut_is_sync() {
        let sess = SessionInfo::new(ConnectionInfo::default(), "test".to_owned());
        let tran = Transaction {
            id: "sessionid".to_owned(),
            ..Default::default()
        };
        let cfg = Config::default();
        let sut = SpfService::new(cfg);
        let fut = sut.send_mail(&sess, tran);
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
