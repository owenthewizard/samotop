pub mod default;
pub mod dirmail;

#[cfg(feature = "spf")]
pub mod spf;

use crate::common::*;
use crate::model::io::Connection;
use crate::model::mail::*;
use crate::model::smtp::SmtpExtension;
use crate::model::Error;
use crate::service::mail::default::DefaultMailService;

pub trait MailSetup<S> {
    type Output: NamedService + EsmtpService + MailGuard + MailQueue;
    fn setup(self, service: S) -> Self::Output;
}

/**
The service which implements this trait has a name.
*/
pub trait NamedService {
    fn name(&self) -> &str;
}

/**
The service which implements this trait has a name.
*/
pub trait EsmtpService {
    fn extend(&self, connection: &mut Connection);
}

/**
A mail guard can be queried whether a recepient is accepted on which address.
*/
pub trait MailGuard {
    type Future: Future<Output = AcceptRecipientResult>;
    fn accept(&self, request: AcceptRecipientRequest) -> Self::Future;
}

/**
A mail queue allows us to queue an e-mail.
For a given mail envelope it produces a Sink that can receive mail data.
Once the sink is closed successfully, the mail is queued.
*/
pub trait MailQueue {
    type Mail: Sink<bytes::Bytes, Error = Error>;
    type MailFuture: Future<Output = Option<Self::Mail>>;
    fn mail(&self, envelope: Envelope) -> Self::MailFuture;
}

impl NamedService for String {
    fn name(&self) -> &str {
        self.as_str()
    }
}
impl NamedService for &str {
    fn name(&self) -> &str {
        self
    }
}

impl<T> NamedService for &T
where
    T: NamedService,
{
    fn name(&self) -> &str {
        T::name(self)
    }
}

impl<T> EsmtpService for &T
where
    T: EsmtpService,
{
    fn extend(&self, conn: &mut Connection) {
        T::extend(self, conn)
    }
}

impl<T> MailGuard for &T
where
    T: MailGuard,
{
    type Future = T::Future;
    fn accept(&self, request: AcceptRecipientRequest) -> Self::Future {
        T::accept(self, request)
    }
}

impl<T> MailQueue for &T
where
    T: MailQueue,
{
    type Mail = T::Mail;
    type MailFuture = T::MailFuture;
    fn mail(&self, envelope: Envelope) -> Self::MailFuture {
        T::mail(self, envelope)
    }
}

#[derive(Clone, Debug)]
pub struct CompositeMailService<NS, ES, GS, QS> {
    named: NS,
    extend: ES,
    guard: GS,
    queue: QS,
}

impl Default
    for CompositeMailService<
        DefaultMailService,
        DefaultMailService,
        DefaultMailService,
        DefaultMailService,
    >
{
    fn default() -> Self {
        CompositeMailService {
            named: DefaultMailService,
            extend: DefaultMailService,
            guard: DefaultMailService,
            queue: DefaultMailService,
        }
    }
}

impl<NS, ES, GS, QS> CompositeMailService<NS, ES, GS, QS> {
    pub fn using<MS: MailSetup<Self>>(self, setup: MS) -> MS::Output {
        setup.setup(self)
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

impl<NS, ES, GS, QS> NamedService for CompositeMailService<NS, ES, GS, QS>
where
    NS: NamedService,
{
    fn name(&self) -> &str {
        self.named.name()
    }
}

impl<NS, ES, GS, QS> EsmtpService for CompositeMailService<NS, ES, GS, QS>
where
    ES: EsmtpService,
{
    fn extend(&self, connection: &mut Connection) {
        self.extend.extend(connection)
    }
}

impl<NS, ES, GS, QS> MailGuard for CompositeMailService<NS, ES, GS, QS>
where
    GS: MailGuard,
{
    type Future = GS::Future;
    fn accept(&self, request: AcceptRecipientRequest) -> Self::Future {
        self.guard.accept(request)
    }
}

impl<NS, ES, GS, QS> MailQueue for CompositeMailService<NS, ES, GS, QS>
where
    QS: MailQueue,
{
    type Mail = QS::Mail;
    type MailFuture = QS::MailFuture;
    fn mail(&self, envelope: Envelope) -> Self::MailFuture {
        self.queue.mail(envelope)
    }
}
