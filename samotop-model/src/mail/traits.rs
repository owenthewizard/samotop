use crate::{common::*, parser::Parser};
use crate::{io::tls::TlsProvider, mail::*};
use std::fmt::Debug;

pub trait MailService:
    TlsProvider + ParserProvider + EsmtpService + MailGuard + MailDispatch
{
}
impl<T> MailService for T where
    T: TlsProvider + ParserProvider + EsmtpService + MailGuard + MailDispatch
{
}

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
pub trait EsmtpService: Debug {
    fn prepare_session(&self, session: &mut SessionInfo);
}

/**
A mail guard can be queried whether a recepient is accepted on which address.
*/
pub trait MailGuard: Debug {
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
pub trait MailDispatch: Debug {
    fn send_mail<'a, 's, 'f>(
        &'a self,
        session: &'s SessionInfo,
        transaction: Transaction,
    ) -> S2Fut<'f, DispatchResult>
    where
        'a: 'f,
        's: 'f;
}

pub trait ParserProvider: Debug {
    fn get_parser_for_data(&self) -> Box<dyn Parser + Sync + Send>;
    fn get_parser_for_commands(&self) -> Box<dyn Parser + Sync + Send>;
}

impl<T> EsmtpService for Arc<T>
where
    T: EsmtpService,
{
    fn prepare_session(&self, session: &mut SessionInfo) {
        T::prepare_session(self, session)
    }
}

impl<T> ParserProvider for Arc<T>
where
    T: ParserProvider,
{
    fn get_parser_for_data(&self) -> Box<dyn Parser + Sync + Send> {
        T::get_parser_for_data(self)
    }
    fn get_parser_for_commands(&self) -> Box<dyn Parser + Sync + Send> {
        T::get_parser_for_commands(self)
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

impl<T> TlsProvider for Arc<T>
where
    T: TlsProvider,
{
    fn get_tls_upgrade(&self) -> Option<Box<dyn crate::io::tls::TlsUpgrade>> {
        T::get_tls_upgrade(self)
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
        builder.dispatch.insert(0, Box::new(NullDispatch))
    }
}

let mail_svc = Builder::default().using(NoDispatch);

```
*/
pub trait MailSetup: std::fmt::Debug {
    fn setup(self, builder: &mut Builder);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestSetup;

    impl MailSetup for TestSetup {
        fn setup(self, builder: &mut Builder) {
            builder
                .dispatch
                .insert(0, Box::new(DebugMailService::default()))
        }
    }

    #[test]
    fn test_setup() {
        let setup = TestSetup;
        let mut builder = Builder::default();
        setup.setup(&mut builder);
        hungry(builder);
    }
    #[test]
    fn test_using() {
        let setup = TestSetup;
        let builder = Builder::default();
        let composite = builder.using(setup);
        hungry(composite);
    }

    fn hungry(_svc: impl MailService + Send + Sync + 'static) {}
}
