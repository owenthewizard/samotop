use crate::mail::Builder;

/**
Can set up the given mail services.

```
# use samotop_core::mail::*;
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
    use crate::mail::{DebugMailService, MailService};

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
