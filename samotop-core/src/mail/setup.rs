use crate::mail::Configuration;

/**
Can set up the given mail services.

```
# use samotop_core::mail::*;
/// This mail setup replaces dispatch service with default. No mail will be sent.
#[derive(Clone, Debug)]
struct NoDispatch;

impl MailSetup for NoDispatch
{
    fn setup(self, config: &mut Configuration) {
        config.dispatch.clear();
        config.dispatch.insert(0, Box::new(NullDispatch))
    }
}

let mail_svc = Builder::default().using(NoDispatch);

```
*/
pub trait MailSetup: std::fmt::Debug {
    fn setup(self, config: &mut Configuration);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mail::{Builder, DebugMailService, MailService, Service};

    #[derive(Debug)]
    struct TestSetup;

    impl MailSetup for TestSetup {
        fn setup(self, config: &mut Configuration) {
            config
                .dispatch
                .insert(0, Box::new(DebugMailService::default()))
        }
    }

    #[test]
    fn test_setup() {
        let setup = TestSetup;
        let mut config = Configuration::default();
        setup.setup(&mut config);
        hungry(Service::new(config));
    }

    #[test]
    fn test_using() {
        let setup = TestSetup;
        let builder = Builder::default();
        let composite = builder.using(setup).build();
        hungry(composite);
    }

    fn hungry(_svc: impl MailService + Send + Sync + 'static) {}
}
