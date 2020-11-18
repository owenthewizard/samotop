use crate::common::*;
use crate::mail::*;

pub trait MailService: EsmtpService + MailGuard + MailDispatch {}
impl<T> MailService for T where T: EsmtpService + MailGuard + MailDispatch {}

/**
The service which implements this trait delivers ESMTP extensions.

```
# use samotop_model::smtp::*;
# use samotop_model::mail::*;
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
    fn add_recipient<'a, 'f>(
        &'a self,
        request: AddRecipientRequest,
    ) -> S2Fut<'f, AddRecipientResult>
    where
        'a: 'f;
    fn start_mail<'a, 's, 'f>(
        &'a self,
        session: &'s SessionInfo,
        request: StartMailRequest,
    ) -> S2Fut<'f, StartMailResult>
    where
        'a: 'f,
        's: 'f;
}

/**
A mail dispatch allows us to dispatch an e-mail.
For a given mail transacton it produces a Write sink that can receive mail data.
Once the sink is closed successfully, the mail is dispatched.
*/
pub trait MailDispatch {
    fn send_mail<'a, 's, 'f>(
        &'a self,
        session: &'s SessionInfo,
        transaction: Transaction,
    ) -> S2Fut<'f, DispatchResult>
    where
        'a: 'f,
        's: 'f;
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
    fn add_recipient<'a, 'f>(
        &'a self,
        request: AddRecipientRequest,
    ) -> S2Fut<'f, AddRecipientResult>
    where
        'a: 'f,
    {
        T::add_recipient(self, request)
    }
    fn start_mail<'a, 's, 'f>(
        &'a self,
        session: &'s SessionInfo,
        request: StartMailRequest,
    ) -> S2Fut<'f, StartMailResult>
    where
        'a: 'f,
        's: 'f,
    {
        T::start_mail(self, session, request)
    }
}

impl<T> MailDispatch for Arc<T>
where
    T: MailDispatch,
{
    fn send_mail<'a, 's, 'f>(
        &'a self,
        session: &'s SessionInfo,
        transaction: Transaction,
    ) -> S2Fut<'f, DispatchResult>
    where
        'a: 'f,
        's: 'f,
    {
        T::send_mail(self, session, transaction)
    }
}

/**
Can set up the given mail services.

```
# use samotop_model::mail::*;
/// This mail setup replaces dispatch service with default. No mail will be sent.
#[derive(Clone, Debug)]
struct NoDispatch;

impl MailSetup for NoDispatch
{
    fn setup(self, builder: &mut Builder) {
        builder.dispatch.clear();
        builder.dispatch.push(Box::new(DefaultMailService::default()))
    }
}

let mail_svc = Builder::default().using(NoDispatch);

```
*/
pub trait MailSetup {
    fn setup(self, builder: &mut Builder);
}

#[cfg(Test)]
mod tests {
    use super::*;
    struct TestSetup;

    impl<ES, GS, DS> MailSetup<ES, GS, DS> for TestSetup
    where
        ES: EsmtpService,
        GS: MailGuard,
        DS: MailDispatch,
    {
        type Output = composite::CompositeMailService<NS, ES, GS, default::DefaultMailService>;
        fn setup(self, extend: ES, guard: GS, _dispatch: DS) -> Self::Output {
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
