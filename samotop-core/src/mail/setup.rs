use crate::{
    mail::{MailDispatch, MailGuard},
    smtp::{EsmtpService, Interpret},
};

/**
Can set up the given mail services.

```
# use samotop_core::mail::*;
/// This mail setup replaces dispatch service with default. No mail will be sent.
#[derive(Clone, Debug)]
struct NoDispatch;

impl<T: AcceptsDispatch> MailSetup<T> for NoDispatch
{
    fn setup(self, config: &mut T) {
        config.wrap_dispatches(|_| NullDispatch)
    }
}

let mail_svc = Builder + NoDispatch;

```
*/
pub trait MailSetup<T>: std::fmt::Debug {
    fn setup(self, config: &mut T);
}

pub trait AcceptsInterpret {
    fn add_interpret<T: Interpret + Send + Sync + 'static>(&mut self, interpret: T);
    fn add_interpret_fallback<T: Interpret + Send + Sync + 'static>(&mut self, interpret: T);
    fn wrap_interprets<T, F>(&mut self, wrap: F)
    where
        T: Interpret + Send + Sync + 'static,
        F: Fn(Box<dyn Interpret + Send + Sync>) -> T;
}
pub trait AcceptsEsmtp {
    fn add_esmtp<T: EsmtpService + Send + Sync + 'static>(&mut self, item: T);
    fn add_esmtp_fallback<T: EsmtpService + Send + Sync + 'static>(&mut self, item: T);
    fn wrap_esmtps<T, F>(&mut self, wrap: F)
    where
        T: EsmtpService + Send + Sync + 'static,
        F: Fn(Box<dyn EsmtpService + Send + Sync>) -> T;
}
pub trait AcceptsGuard {
    fn add_guard<T: MailGuard + Send + Sync + 'static>(&mut self, item: T);
    fn add_guard_fallback<T: MailGuard + Send + Sync + 'static>(&mut self, item: T);
    fn wrap_guards<T, F>(&mut self, wrap: F)
    where
        T: MailGuard + Send + Sync + 'static,
        F: Fn(Box<dyn MailGuard + Send + Sync>) -> T;
}
pub trait AcceptsDispatch {
    fn add_dispatch<T: MailDispatch + Send + Sync + 'static>(&mut self, item: T);
    fn add_dispatch_fallback<T: MailDispatch + Send + Sync + 'static>(&mut self, item: T);
    fn wrap_dispatches<T, F>(&mut self, wrap: F)
    where
        T: MailDispatch + Send + Sync + 'static,
        F: Fn(Box<dyn MailDispatch + Send + Sync>) -> T;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mail::{Builder, Debug, MailService};

    #[derive(Debug)]
    struct TestSetup;

    impl<T: AcceptsDispatch> MailSetup<T> for TestSetup {
        fn setup(self, config: &mut T) {
            config.add_dispatch(Debug::default())
        }
    }

    #[test]
    fn test_composition() {
        let composite = Builder + TestSetup;
        hungry(composite.build());
    }

    fn hungry(_svc: impl MailService + Send + Sync + 'static) {}
}
