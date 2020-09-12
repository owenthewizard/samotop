pub mod composite;
pub mod default;

use crate::common::*;
use crate::model::mail::*;
use composite::CompositeMailService;
use composite::IntoComponents;

pub trait MailServiceBuilder: IntoComponents {
    fn using<MS: MailSetup<Self::Named, Self::Esmtp, Self::Guard, Self::Queue>>(
        self,
        setup: MS,
    ) -> MS::Output;
}

impl<T> MailServiceBuilder for T
where
    T: IntoComponents,
{
    fn using<MS: MailSetup<T::Named, T::Esmtp, T::Guard, T::Queue>>(self, setup: MS) -> MS::Output {
        let (n, e, h, q) = self.into_components();
        setup.setup(n, e, h, q)
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
        ("my.name.example.com", extend, guard, queue).into()
    }
}

let mail_svc = default::DefaultMailService.using(MyMail);

```
*/
pub trait MailSetup<NS, ES, GS, QS> {
    type Output: MailService;
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
pub trait NamedService {
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
pub trait EsmtpService {
    fn extend(&self, connection: &mut SessionInfo);
}

/**
A mail guard can be queried whether a recepient is accepted on which address.
*/
pub trait MailGuard {
    type RecipientFuture: Future<Output = AddRecipientResult> + Send + Sync + 'static;
    type SenderFuture: Future<Output = StartMailResult> + Send + Sync + 'static;
    fn add_recipient(&self, request: AddRecipientRequest) -> Self::RecipientFuture;
    fn start_mail(&self, request: StartMailRequest) -> Self::SenderFuture;
}

/**
A mail queue allows us to queue an e-mail.
For a given mail envelope it produces a Sink that can receive mail data.
Once the sink is closed successfully, the mail is queued.
*/
pub trait MailQueue {
    type Mail: Write + Send + Sync + 'static;
    type MailFuture: Future<Output = Option<Self::Mail>> + Send + Sync + 'static;
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

impl<NS, ES, GS, QS> MailSetup<NS, ES, GS, QS> for String
where
    NS: NamedService,
    ES: EsmtpService,
    GS: MailGuard,
    QS: MailQueue,
{
    type Output = CompositeMailService<Self, ES, GS, QS>;
    fn setup(self, _named: NS, extend: ES, guard: GS, queue: QS) -> Self::Output {
        (self, extend, guard, queue).into()
    }
}

impl<NS, ES, GS, QS> MailSetup<NS, ES, GS, QS> for &str
where
    NS: NamedService,
    ES: EsmtpService,
    GS: MailGuard,
    QS: MailQueue,
{
    type Output = CompositeMailService<Self, ES, GS, QS>;
    fn setup(self, _named: NS, extend: ES, guard: GS, queue: QS) -> Self::Output {
        (self, extend, guard, queue).into()
    }
}

impl<T> NamedService for Arc<T>
where
    T: NamedService,
{
    fn name(&self) -> &str {
        T::name(self)
    }
}

impl<T> EsmtpService for Arc<T>
where
    T: EsmtpService,
{
    fn extend(&self, conn: &mut SessionInfo) {
        T::extend(self, conn)
    }
}

impl<T> MailGuard for Arc<T>
where
    T: MailGuard,
{
    type RecipientFuture = T::RecipientFuture;
    type SenderFuture = T::SenderFuture;
    fn add_recipient(&self, request: AddRecipientRequest) -> Self::RecipientFuture {
        T::add_recipient(self, request)
    }
    fn start_mail(&self, request: StartMailRequest) -> Self::SenderFuture {
        T::start_mail(self, request)
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
