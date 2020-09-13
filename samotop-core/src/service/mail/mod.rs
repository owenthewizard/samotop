pub mod composite;
pub mod default;

use crate::common::*;
use crate::model::mail::*;
use composite::IntoComponents;

pub trait MailServiceBuilder: IntoComponents {
    fn using<MS: MailSetup<Self::Esmtp, Self::Guard, Self::Queue>>(self, setup: MS) -> MS::Output;
}

impl<T> MailServiceBuilder for T
where
    T: IntoComponents,
{
    fn using<MS: MailSetup<T::Esmtp, T::Guard, T::Queue>>(self, setup: MS) -> MS::Output {
        let (e, h, q) = self.into_components();
        setup.setup(e, h, q)
    }
}

pub trait MailService: EsmtpService + MailGuard + MailQueue {}
impl<T> MailService for T where T: EsmtpService + MailGuard + MailQueue {}

/**
Can set up the given mail services.

```
# use samotop_core::service::mail::*;
/// This mail setup replaces queue service with default. No mail will be sent.
#[derive(Clone, Debug)]
struct NoQueue;

impl<ES, GS, QS> MailSetup<ES, GS, QS> for NoQueue
where
    ES: EsmtpService,
    GS: MailGuard,
    QS: MailQueue,
{
    type Output = composite::CompositeMailService<ES, GS, default::DefaultMailService>;
    fn setup(self, extend: ES, guard: GS, _queue: QS) -> Self::Output {
        (extend, guard, default::DefaultMailService::default()).into()
    }
}

let mail_svc = default::DefaultMailService::default().using(NoQueue);

```
*/
pub trait MailSetup<ES, GS, QS> {
    type Output: MailService;
    fn setup(self, extend: ES, guard: GS, queue: QS) -> Self::Output;
}

/**
The service which implements this trait delivers ESMTP extensions.

```
# use samotop_core::service::mail::*;
# use samotop_core::model::smtp::*;
# use samotop_core::model::mail::*;
/// This mail service canhabdle 8-bit MIME
#[derive(Clone, Debug)]
pub struct EnableEightBit<T>(T);

impl<T> EsmtpService for EnableEightBit<T>
where
    T: EsmtpService,
{
    fn prepare_session(&self, session: &mut SessionInfo) {
        self.0.prepare_session(session);
        session
            .extensions
            .enable(&extension::EIGHTBITMIME);
    }
}
```
*/
pub trait EsmtpService {
    fn prepare_session(&self, session: &mut SessionInfo);
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

impl<T> EsmtpService for Arc<T>
where
    T: EsmtpService,
{
    fn prepare_session(&self, session: &mut SessionInfo) {
        T::prepare_session(self, session)
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

    impl<ES, GS, QS> MailSetup<ES, GS, QS> for TestSetup
    where
        ES: EsmtpService,
        GS: MailGuard,
        QS: MailQueue,
    {
        type Output = composite::CompositeMailService<NS, ES, GS, default::DefaultMailService>;
        fn setup(self, extend: ES, guard: GS, _queue: QS) -> Self::Output {
            (extend, guard, default::DefaultMailService)
        }
    }

    #[test]
    fn test_setup() {
        let setup = TestSetup;
        let svc = default::DefaultMailService;
        let composite = setup.setup(svc.clone(), svc.clone(), svc);
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
