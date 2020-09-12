mod lookup;

use super::composite::*;
use super::*;
use crate::common::*;
use crate::model::mail::Envelope;
use crate::model::smtp::*;
use lookup::*;
pub use viaspf::Config;
use viaspf::{evaluate_spf, SpfResult};

pub fn provide_viaspf() -> Provider<Config> {
    Provider(Config::default())
}

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

impl<NS, ES, GS, QS> MailSetup<NS, ES, GS, QS> for Provider<Config>
where
    NS: NamedService,
    ES: EsmtpService,
    GS: MailGuard,
    QS: MailQueue,
{
    type Output = CompositeMailService<NS, ES, GS, SpfService<QS>>;
    fn setup(self, named: NS, extend: ES, guard: GS, queue: QS) -> Self::Output {
        (named, extend, guard, SpfService::new(queue, self.0)).into()
    }
}

impl<T: MailQueue> MailQueue for SpfService<T> {
    type Mail = T::Mail;
    type MailFuture = MailQueueFut<T::MailFuture>;
    fn mail(&self, envelope: Envelope) -> Self::MailFuture {
        MailQueueFut {
            config: self.config.clone(),
            inner: self.inner.mail(envelope.clone()),
            envelope,
        }
    }
}

#[pin_project]
pub struct MailQueueFut<T> {
    #[pin]
    inner: T,
    config: Config,
    envelope: Envelope,
}

impl<F, T: Future<Output = Option<F>>> Future for MailQueueFut<T> {
    type Output = T::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // TODO: improve privacy - a) encrypt DNS, b) do DNS servers need to know who is receiving mail from whom?
        // TODO: convert to async
        let proj = self.project();
        let sender = match proj.envelope.mail.as_ref().map(|m| m.from()) {
            None | Some(SmtpPath::Null) | Some(SmtpPath::Postmaster) => String::new(),
            Some(SmtpPath::Direct(SmtpAddress::Mailbox(_account, host))) => host.domain(),
            Some(SmtpPath::Relay(_path, SmtpAddress::Mailbox(_account, host))) => host.domain(),
        };
        let peer_ip = match proj
            .envelope
            .session
            .connection
            .peer_addr
            .map(|addr| addr.ip())
        {
            None => std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
            Some(ip) => ip,
        };
        let helo_domain = match proj
            .envelope
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
                Poll::Ready(None)
            }
            result => {
                trace!("mail OK with SPF result: {}", result);
                // TODO: Add SPF result to mail headers
                proj.inner.poll(cx)
            }
        }
    }
}
