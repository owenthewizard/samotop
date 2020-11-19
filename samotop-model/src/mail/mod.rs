mod builder;
mod default;
mod traits;
mod types;

pub use self::builder::*;
pub use self::default::*;
pub use self::traits::*;
pub use self::types::*;

#[cfg(test)]
mod tests {
    use super::*;
    struct TestSetup;

    impl MailSetup for TestSetup {
        fn setup(self, builder: &mut Builder) {
            builder
                .dispatch
                .insert(0,Box::new(DefaultMailService::default()));
        }
    }

    #[test]
    fn test_setup() {
        let setup = TestSetup;
        let mut svc = Builder::default();
        setup.setup(&mut svc);
        hungry(svc);
    }
    #[test]
    fn test_using() {
        let setup = TestSetup;
        let svc = Builder::default();
        let svc = svc.using(setup);
        hungry(svc);
    }

    fn hungry(_svc: impl MailService + Send + Sync + 'static) {}
}
