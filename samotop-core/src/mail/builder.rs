use crate::{
    common::*,
    mail::{EsmtpService, MailDispatch, MailGuard, MailSetup, Service},
    smtp::Interpret,
};

/// Builds MailService from components
#[derive(Default)]
pub struct Builder {
    config: Configuration,
}

/// Service builder configuration
#[derive(Debug)]
pub struct Configuration {
    /// ID used for identifying this instance in logs
    pub logging_id: String,
    pub interpret: Vec<Box<dyn Interpret + Send + Sync>>,
    pub dispatch: Vec<Box<dyn MailDispatch + Sync + Send + 'static>>,
    pub guard: Vec<Box<dyn MailGuard + Sync + Send + 'static>>,
    pub esmtp: Vec<Box<dyn EsmtpService + Sync + Send + 'static>>,
}

impl Builder {
    /// Use a given MailSetup to build a MailService.
    ///
    /// See MailSetup for examples.
    pub fn using(mut self, setup: impl MailSetup) -> Self {
        trace!(
            "Service builder {} using setup {:?}",
            self.config.logging_id,
            setup
        );
        setup.setup(&mut self.config);
        self
    }
    /// Finalize and produce the MailService.
    pub fn build(self) -> Service {
        Service::new(self.config)
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            logging_id: time_based_id(),
            interpret: Default::default(),
            dispatch: Default::default(),
            guard: Default::default(),
            esmtp: Default::default(),
        }
    }
}
