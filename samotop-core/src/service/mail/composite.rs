use super::*;
use crate::model::io::Connection;
use crate::model::mail::*;

pub type CompositeMailService<NS, ES, GS, QS> = (NS, ES, GS, QS);
/*
#[derive(Clone, Debug)]
pub struct CompositeMailService<NS, ES, GS, QS> {
    named: NS,
    extend: ES,
    guard: GS,
    queue: QS,
}

impl Default
    for CompositeMailService<
        Arc<DefaultMailService>,
        Arc<DefaultMailService>,
        Arc<DefaultMailService>,
        Arc<DefaultMailService>,
    >
{
    fn default() -> Self {
        let svc = Arc::new(DefaultMailService);
        CompositeMailService {
            named: svc.clone(),
            extend: svc.clone(),
            guard: svc.clone(),
            queue: svc,
        }
    }
}

impl<NS, ES, GS, QS> CompositeMailService<NS, ES, GS, QS> {
    pub fn using<MS: MailSetup<NS, ES, GS, QS>>(self, setup: MS) -> MS::Output {
        let CompositeMailService {
            named,
            extend,
            guard,
            queue,
        } = self;
        setup.setup(named, extend, guard, queue)
    }
    pub fn from_components(named: NS, extend: ES, guard: GS, queue: QS) -> Self {
        CompositeMailService {
            named,
            extend,
            guard,
            queue,
        }
    }
    pub fn into_components(self) -> (NS, ES, GS, QS) {
        let CompositeMailService {
            named,
            extend,
            guard,
            queue,
        } = self;
        (named, extend, guard, queue)
    }
    pub fn replace_guard<T, F>(self, replacement: F) -> CompositeMailService<NS, ES, T, QS>
    where
        T: MailGuard,
        F: FnOnce(GS) -> T,
    {
        let (named, extend, guard, queue) = self.into_components();
        let guard = replacement(guard);
        CompositeMailService::from_components(named, extend, guard, queue)
    }
    pub fn with_guard<T: MailGuard>(self, guard: T) -> CompositeMailService<NS, ES, T, QS> {
        self.replace_guard(|_| guard)
    }
    pub fn replace_queue<T, F>(self, replacement: F) -> CompositeMailService<NS, ES, GS, T>
    where
        T: MailQueue,
        F: FnOnce(QS) -> T,
    {
        let (named, extend, guard, queue) = self.into_components();
        let queue = replacement(queue);
        CompositeMailService::from_components(named, extend, guard, queue)
    }
    pub fn with_queue<T: MailQueue>(self, queue: T) -> CompositeMailService<NS, ES, GS, T> {
        self.replace_queue(|_| queue)
    }
    pub fn replace_esmtp<T, F>(self, replacement: F) -> CompositeMailService<NS, T, GS, QS>
    where
        T: EsmtpService,
        F: FnOnce(ES) -> T,
    {
        let (named, extend, guard, queue) = self.into_components();
        let extend = replacement(extend);
        CompositeMailService::from_components(named, extend, guard, queue)
    }
    pub fn with_esmtp<T: EsmtpService>(self, esmtp: T) -> CompositeMailService<NS, T, GS, QS> {
        self.replace_esmtp(|_| esmtp)
    }
    pub fn replace_name<T, F>(self, replacement: F) -> CompositeMailService<T, ES, GS, QS>
    where
        T: NamedService,
        F: FnOnce(NS) -> T,
    {
        let (named, extend, guard, queue) = self.into_components();
        let named = replacement(named);
        CompositeMailService::from_components(named, extend, guard, queue)
    }
    pub fn with_name<T: NamedService>(self, named: T) -> CompositeMailService<T, ES, GS, QS> {
        self.replace_name(|_| named)
    }
}
*/

impl<NS, ES, GS, QS> NamedService for CompositeMailService<NS, ES, GS, QS>
where
    NS: NamedService,
    ES: EsmtpService,
    GS: MailGuard,
    QS: MailQueue,
{
    fn name(&self) -> &str {
        self.0.name()
    }
}

impl<NS, ES, GS, QS> EsmtpService for CompositeMailService<NS, ES, GS, QS>
where
    NS: NamedService,
    ES: EsmtpService,
    GS: MailGuard,
    QS: MailQueue,
{
    fn extend(&self, connection: &mut Connection) {
        self.1.extend(connection)
    }
}

impl<NS, ES, GS, QS> MailGuard for CompositeMailService<NS, ES, GS, QS>
where
    NS: NamedService,
    ES: EsmtpService,
    GS: MailGuard,
    QS: MailQueue,
{
    type Future = GS::Future;
    fn accept(&self, request: AcceptRecipientRequest) -> Self::Future {
        self.2.accept(request)
    }
}

impl<NS, ES, GS, QS> MailQueue for CompositeMailService<NS, ES, GS, QS>
where
    NS: NamedService,
    ES: EsmtpService,
    GS: MailGuard,
    QS: MailQueue,
{
    type Mail = QS::Mail;
    type MailFuture = QS::MailFuture;
    fn mail(&self, envelope: Envelope) -> Self::MailFuture {
        self.3.mail(envelope)
    }
}
