use crate::mail::Configuration;

/**
Can set up the given mail services.

```
# use samotop_core::mail::*;
/// This mail setup replaces dispatch service with default. No mail will be sent.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
struct NoDispatch;

impl MailSetup for NoDispatch
{
    fn setup(self, config: &mut Configuration) {
        config.wrap_dispatches(|_| NullDispatch)
    }
}

let mail_svc = Builder + NoDispatch;

```
*/

#[cfg(feature = "serialize")]
pub trait MailSetup: std::fmt::Debug + serde::Serialize + serde::Deserialize<'static> {
    fn setup(self, config: &mut Configuration);
}

#[cfg(not(feature = "serialize"))]
pub trait MailSetup: std::fmt::Debug {
    fn setup(self, config: &mut Configuration);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mail::*;

    #[derive(Debug)]
    #[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
    struct TestSetup;

    impl MailSetup for TestSetup {
        fn setup(self, config: &mut Configuration) {
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
