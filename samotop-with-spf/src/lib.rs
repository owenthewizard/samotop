#[macro_use]
extern crate log;

mod lookup;

use self::lookup::*;
use samotop_model::common::*;
use samotop_model::mail::*;
use samotop_model::smtp::*;
pub use viaspf::Config;
use viaspf::{evaluate_spf, SpfResult};

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

impl MailSetup for Provider<Config> {
    fn setup(self, builder: &mut Builder) {
        builder
            .dispatch
            .insert(0, Box::new(SpfService::new(self.0)));
    }
}

impl MailDispatch for SpfService {
    fn send_mail<'a, 's, 'f>(
        &'a self,
        session: &'s SessionInfo,
        transaction: Transaction,
    ) -> S2Fut<'f, DispatchResult>
    where
        'a: 'f,
        's: 'f,
    {
        let peer_addr = match session.connection.peer_addr.map(|addr| addr.ip()) {
            None => std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
            Some(ip) => ip,
        };
        let peer_name = match session.smtp_helo.as_ref().map(|m| m.host().domain()) {
            None => String::new(),
            Some(s) => s,
        };
        let sender = match transaction.mail.as_ref().map(|m| m.path()) {
            None | Some(SmtpPath::Null) | Some(SmtpPath::Postmaster) => String::new(),
            Some(SmtpPath::Direct(SmtpAddress::Mailbox(_account, host))) => host.domain(),
            Some(SmtpPath::Relay(_path, SmtpAddress::Mailbox(_account, host))) => host.domain(),
        };
        let fut = async move {
            // TODO: improve privacy - a) encrypt DNS, b) do DNS servers need to know who is receiving mail from whom?
            // TODO: convert to async
            let evaluation = evaluate_spf(
                &TrustDnsResolver::default(),
                &self.config,
                peer_addr,
                sender.as_str(),
                peer_name.as_str(),
            );
            match evaluation.result {
                SpfResult::Fail(explanation) => {
                    debug!("mail rejected due to SPF fail: {}", explanation);
                    Err(DispatchError::Refused)
                }
                result => {
                    trace!("mail OK with SPF result: {}", result);
                    // TODO: Add SPF result to mail headers
                    Ok(transaction)
                }
            }
        };

        Box::pin(fut)
    }
}

#[cfg(test)]
mod tests {
    use samotop_model::io::ConnectionInfo;

    use super::*;

    #[test]
    fn default_mail_fut_is_sync() {
        let sess = SessionInfo::new(ConnectionInfo::new(None, None), "test".to_owned());
        let tran = Transaction {
            id: "sessionid".to_owned(),
            ..Default::default()
        };
        let cfg = Config::default();
        let sut = SpfService::new(cfg);
        let fut = sut.send_mail(&sess, tran);
        is_sync(fut);
    }

    #[test]
    fn config_is_sync() {
        let cfg = Config::default();
        is_sync(cfg);
    }

    fn is_sync<T: Sync>(_subject: T) {}
}
