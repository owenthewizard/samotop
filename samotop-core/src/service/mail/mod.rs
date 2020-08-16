pub mod composite;
pub mod default;

use crate::common::*;
use crate::model::io::Connection;
use crate::model::mail::*;
use crate::model::Error;
use composite::CompositeMailService;
use std::ops::Deref;

pub trait MailServiceBuilder {
    type Named: NamedService;
    type Esmtp: EsmtpService;
    type Guard: MailGuard;
    type Queue: MailQueue;
    fn using<MS: MailSetup<Self::Named, Self::Esmtp, Self::Guard, Self::Queue>>(
        self,
        setup: MS,
    ) -> MS::Output;
}

impl<T> MailServiceBuilder for T
where
    T: Clone + MailService,
{
    type Named = Self;
    type Esmtp = Self;
    type Guard = Self;
    type Queue = Self;
    fn using<MS: MailSetup<Self, Self, Self, Self>>(self, setup: MS) -> MS::Output {
        setup.setup(self.clone(), self.clone(), self.clone(), self)
    }
}

pub trait MailService: NamedService + EsmtpService + MailGuard + MailQueue {}
impl<T> MailService for T where T: NamedService + EsmtpService + MailGuard + MailQueue {}

/**
Can set up the given mail services.

```
# use samotop_core::service::mail::*;
/// This mail service has a very special name
#[derive(Clone, Debug)]
struct MyMail;

impl<NS, ES, GS, QS> MailSetup<NS, ES, GS, QS> for MyMail
where
    NS: NamedService,
    ES: EsmtpService,
    GS: MailGuard,
    QS: MailQueue,
{
    type Output = composite::CompositeMailService<&'static str, ES, GS, QS>;
    fn setup(self, _named: NS, extend: ES, guard: GS, queue: QS) -> Self::Output {
        ("my.name.example.com", extend, guard, queue)
    }
}

let mail_svc = default::DefaultMailService.using(MyMail);

```
*/
pub trait MailSetup<NS, ES, GS, QS> {
    type Output: MailService + Send + Sync;
    fn setup(self, named: NS, extend: ES, guard: GS, queue: QS) -> Self::Output;
}

/**
The service which implements this trait has a name.

```
# use samotop_core::service::mail::*;
/// This mail service has a very special names
#[derive(Clone, Debug)]
struct MyNameMail;

impl NamedService for MyNameMail {
    fn name(&self) -> &str {
        "my.name.example.com"
    }
}
```
*/
pub trait NamedService: Send + Sync {
    fn name(&self) -> &str;
}

/**
The service which implements this trait delivers ESMTP extensions.

```
# use samotop_core::service::mail::*;
# use samotop_core::model::io::Connection;
# use samotop_core::model::smtp::SmtpExtension;
/// This mail service canhabdle 8-bit MIME
#[derive(Clone, Debug)]
pub struct EnableEightBit<T>(T);

impl<T> EsmtpService for EnableEightBit<T>
where
    T: EsmtpService,
{
    fn extend(&self, connection: &mut Connection) {
        self.0.extend(connection);
        connection
            .extensions_mut()
            .enable(SmtpExtension::EIGHTBITMIME);
    }
}
```
*/
pub trait EsmtpService: Send + Sync {
    fn extend(&self, connection: &mut Connection);
}

/**
A mail guard can be queried whether a recepient is accepted on which address.
*/
pub trait MailGuard: Send + Sync {
    type Future: Future<Output = AcceptRecipientResult>;
    fn accept(&self, request: AcceptRecipientRequest) -> Self::Future;
}

/**
A mail queue allows us to queue an e-mail.
For a given mail envelope it produces a Sink that can receive mail data.
Once the sink is closed successfully, the mail is queued.
*/
pub trait MailQueue: Send + Sync {
    type Mail: Sink<Bytes, Error = Error>;
    type MailFuture: Future<Output = Option<Self::Mail>>;
    fn mail(&self, envelope: Envelope) -> Self::MailFuture;
}

impl NamedService for &str {
    fn name(&self) -> &str {
        self
    }
}
impl NamedService for String {
    fn name(&self) -> &str {
        self.as_str()
    }
}

impl<T, NS, ES, GS, QS> MailSetup<NS, ES, GS, QS> for T
where
    T: NamedService,
    NS: NamedService,
    ES: EsmtpService,
    GS: MailGuard,
    QS: MailQueue,
{
    type Output = CompositeMailService<T, ES, GS, QS>;
    fn setup(self, _named: NS, extend: ES, guard: GS, queue: QS) -> Self::Output {
        (self, extend, guard, queue)
    }
}

impl<T> NamedService for Arc<T>
where
    T: NamedService,
{
    fn name(&self) -> &str {
        T::name(Arc::deref(self))
    }
}

impl<T> EsmtpService for Arc<T>
where
    T: EsmtpService,
{
    fn extend(&self, conn: &mut Connection) {
        T::extend(self, conn)
    }
}

impl<T> MailGuard for Arc<T>
where
    T: MailGuard,
{
    type Future = T::Future;
    fn accept(&self, request: AcceptRecipientRequest) -> Self::Future {
        T::accept(self, request)
    }
}

impl<T> MailQueue for Arc<T>
where
    T: MailQueue,
{
    type Mail = T::Mail;
    type MailFuture = T::MailFuture;
    fn mail(&self, envelope: Envelope) -> Self::MailFuture {
        T::mail(self, envelope)
    }
}

#[cfg(Test)]
mod tests {
    use super::*;
    struct TestSetup;

    impl<NS, ES, GS, QS> MailSetup<NS, ES, GS, QS> for TestSetup
    where
        NS: NamedService,
        ES: EsmtpService,
        GS: MailGuard,
        QS: MailQueue,
    {
        type Output = composite::CompositeMailService<NS, ES, GS, default::DefaultMailService>;
        fn setup(self, named: NS, extend: ES, guard: GS, _queue: QS) -> Self::Output {
            (named, extend, guard, default::DefaultMailService)
        }
    }

    #[test]
    fn test_setup() {
        let setup = TestSetup;
        let svc = default::DefaultMailService;
        let composite = setup.setup(svc.clone(), svc.clone(), svc.clone(), svc);
        hungry(composite);
    }
    #[test]
    fn test_using() {
        let setup = TestSetup;
        let svc = default::DefaultMailService;
        let composite = svc.using(setup);
        hungry(composite);
    }

    #[test]
    fn test_using_name() {
        let setup = "myname";
        let svc = default::DefaultMailService;
        let composite = svc.using(setup);
        hungry(composite);
    }

    fn hungry(_svc: impl MailService + Send + Sync + Clone + std::fmt::Debug + 'static) {}
}
