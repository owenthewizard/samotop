#[macro_use]
extern crate log;

mod lookup;

use self::lookup::*;
use samotop_core::common::*;
use samotop_core::model::mail::*;
use samotop_core::model::smtp::*;
use samotop_core::service::mail::composite::*;
use samotop_core::service::mail::*;
pub use viaspf::Config;
use viaspf::{evaluate_spf, SpfResult};

pub fn provide_viaspf() -> Provider<Config> {
    Provider(Config::default())
}

#[derive(Clone, Debug, Default)]
pub struct Provider<T>(pub T);

#[derive(Clone, Debug)]
pub struct SpfService<T> {
    inner: T,
    config: Config,
}

impl<T> SpfService<T> {
    pub fn new(inner: T, config: Config) -> Self {
        Self { inner, config }
    }
}

impl<ES, GS, DS> MailSetup<ES, GS, DS> for Provider<Config>
where
    ES: EsmtpService,
    GS: MailGuard,
    DS: MailDispatch,
{
    type Output = CompositeMailService<ES, GS, SpfService<DS>>;
    fn setup(self, extend: ES, guard: GS, dispatch: DS) -> Self::Output {
        (extend, guard, SpfService::new(dispatch, self.0)).into()
    }
}

impl<T: MailDispatch> MailDispatch for SpfService<T> {
    type Mail = T::Mail;
    type MailFuture = MailDispatchFut<T::MailFuture>;
    fn send_mail(&self, transaction: Transaction) -> Self::MailFuture {
        MailDispatchFut {
            config: self.config.clone(),
            inner: self.inner.send_mail(transaction.clone()),
            transaction,
        }
    }
}

#[pin_project]
pub struct MailDispatchFut<T> {
    #[pin]
    inner: T,
    config: Config,
    transaction: Transaction,
}

impl<F, T: Future<Output = DispatchResult<F>>> Future for MailDispatchFut<T> {
    type Output = T::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // TODO: improve privacy - a) encrypt DNS, b) do DNS servers need to know who is receiving mail from whom?
        // TODO: convert to async
        let proj = self.project();
        let sender = match proj.transaction.mail.as_ref().map(|m| m.from()) {
            None | Some(SmtpPath::Null) | Some(SmtpPath::Postmaster) => String::new(),
            Some(SmtpPath::Direct(SmtpAddress::Mailbox(_account, host))) => host.domain(),
            Some(SmtpPath::Relay(_path, SmtpAddress::Mailbox(_account, host))) => host.domain(),
        };
        let peer_ip = match proj
            .transaction
            .session
            .connection
            .peer_addr
            .map(|addr| addr.ip())
        {
            None => std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
            Some(ip) => ip,
        };
        let helo_domain = match proj
            .transaction
            .session
            .smtp_helo
            .as_ref()
            .map(|m| m.host().domain())
        {
            None => String::new(),
            Some(s) => s,
        };
        let evaluation = evaluate_spf(
            &TrustDnsResolver::default(),
            proj.config,
            peer_ip,
            sender.as_str(),
            helo_domain.as_str(),
        );
        match evaluation.result {
            SpfResult::Fail(explanation) => {
                debug!("mail rejected due to SPF fail: {}", explanation);
                Poll::Ready(Err(DispatchError::Refused))
            }
            result => {
                trace!("mail OK with SPF result: {}", result);
                // TODO: Add SPF result to mail headers
                proj.inner.poll(cx)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mail_fut_is_sync() {
        let svc = samotop_core::service::mail::default::DefaultMailService::default();
        let tran = Transaction {
            session: SessionInfo::new(
                samotop_core::model::io::ConnectionInfo::new(None, None),
                "test".to_owned(),
            ),
            id: "sessionid".to_owned(),
            mail: None,
            rcpts: vec![],
        };
        let cfg = Config::default();
        let sut = SpfService::new(svc, cfg);
        let fut = sut.send_mail(tran);
        is_sync(fut);
    }

    #[test]
    fn config_is_sync() {
        let cfg = Config::default();
        is_sync(cfg);
    }

    fn is_sync<T: Sync>(_subject: T) {}
}
