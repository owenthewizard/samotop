use crate::{mail::*, smtp::*};
use std::ops::{Add, AddAssign};

/// Builds MailService from components with a builder pattern.
///
/// Add components with a + sign. The component must implement `MailSetup<T>`
/// and may depend with T on traits the `Configuration` struct implements.
/// Finally call `build()` or `build_with_driver()`
#[derive(Default, Debug)]
pub struct Builder;

/// Composing a mail service with +
impl<T: MailSetup> Add<T> for Builder {
    type Output = BuilderWithConfig;
    /// Add given mail setup to the service configuration
    fn add(self, setup: T) -> Self::Output {
        BuilderWithConfig::default() + setup
    }
}

impl Builder {
    /// Start with empty configuration
    pub fn empty() -> BuilderWithConfig {
        BuilderWithConfig::default()
    }
    /// Use a given MailSetup to build a MailService.
    ///
    /// See MailSetup for examples. Prefer to build with the + sign.
    pub fn using(self, setup: impl MailSetup) -> BuilderWithConfig {
        BuilderWithConfig::default() + setup
    }
    #[cfg(feature = "driver")]
    /// Finalize and produce the MailService.
    pub fn build(self) -> Service {
        BuilderWithConfig::default().build()
    }
}

/// Represents the on-going builder configuration
#[derive(Default)]
pub struct BuilderWithConfig {
    config: Configuration,
}

/// Composing a mail service with +
impl<T: MailSetup> Add<T> for BuilderWithConfig {
    type Output = Self;
    /// Add given mail setup to the service configuration
    fn add(mut self, setup: T) -> Self::Output {
        self += setup;
        self
    }
}
/// Composing a mail service with +=
impl<T: MailSetup> AddAssign<T> for BuilderWithConfig {
    fn add_assign(&mut self, setup: T) {
        trace!(
            "Service builder {} using setup {:?}",
            self.config.id(),
            setup
        );
        setup.setup(&mut self.config)
    }
}

impl BuilderWithConfig {
    /// Use a given MailSetup to build a MailService.
    ///
    /// See MailSetup for examples. Prefer to build with the + sign.
    pub fn using(self, setup: impl MailSetup) -> Self {
        self + setup
    }

    #[cfg(feature = "driver")]
    /// Finalize and produce the MailService.
    pub fn build(self) -> Service {
        self.build_with_driver(crate::smtp::SmtpDriver)
    }

    /// Finalize and produce the MailService.
    pub fn build_with_driver(self, driver: impl Drive + Send + Sync + 'static) -> Service {
        self.config.into_service(driver)
    }
}
