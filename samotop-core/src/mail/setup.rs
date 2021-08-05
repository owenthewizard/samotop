use crate::{
    mail::{MailDispatch, MailGuard},
    smtp::{Interpret, SessionService},
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

pub trait HasId {
    fn id(&self) -> &str;
}

pub trait AcceptsSessionService {
    fn add_first_session_service<T: SessionService + Send + Sync + 'static>(&mut self, item: T);
    fn add_last_session_service<T: SessionService + Send + Sync + 'static>(&mut self, item: T);
    fn wrap_session_service<T, F>(&mut self, wrap: F)
    where
        T: SessionService + Send + Sync + 'static,
        F: FnOnce(Box<dyn SessionService + Send + Sync>) -> T;
}
pub trait AcceptsInterpretter {
    fn add_first_interpretter<T: Interpret + Send + Sync + 'static>(&mut self, item: T);
    fn add_last_interpretter<T: Interpret + Send + Sync + 'static>(&mut self, item: T);
    fn wrap_interpretter<T, F>(&mut self, wrap: F)
    where
        T: Interpret + Send + Sync + 'static,
        F: FnOnce(Box<dyn Interpret + Send + Sync>) -> T;
}
pub trait AcceptsGuard {
    fn add_first_guard<T: MailGuard + Send + Sync + 'static>(&mut self, item: T);
    fn add_last_guard<T: MailGuard + Send + Sync + 'static>(&mut self, item: T);
    fn wrap_guards<T, F>(&mut self, wrap: F)
    where
        T: MailGuard + Send + Sync + 'static,
        F: FnOnce(Box<dyn MailGuard + Send + Sync>) -> T;
}
pub trait AcceptsDispatch {
    fn add_first_dispatch<T: MailDispatch + Send + Sync + 'static>(&mut self, item: T);
    fn add_last_dispatch<T: MailDispatch + Send + Sync + 'static>(&mut self, item: T);
    fn wrap_dispatches<T, F>(&mut self, wrap: F)
    where
        T: MailDispatch + Send + Sync + 'static,
        F: FnOnce(Box<dyn MailDispatch + Send + Sync>) -> T;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mail::*;

    #[derive(Debug)]
    struct TestSetup;

    impl<T: AcceptsDispatch> MailSetup<T> for TestSetup {
        fn setup(self, config: &mut T) {
            config.add_last_dispatch(NullDispatch)
        }
    }

    #[cfg(feature = "driver")]
    #[test]
    fn test_composition() {
        fn hungry(_svc: impl MailService + Send + Sync + 'static) {}
        let composite = Builder + TestSetup;
        hungry(composite.build());
    }
}
