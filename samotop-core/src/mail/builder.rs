use crate::{
    common::*,
    io::tls::{NoTls, TlsProvider},
    mail::{EsmtpService, MailDispatch, MailGuard, MailSetup, Service},
    smtp::{Dummy, Interpret},
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
    pub tls: Box<dyn TlsProvider + Sync + Send + 'static>,
    pub interpretter: Arc<dyn Interpret + Send + Sync>,
    pub dispatch: Vec<Box<dyn MailDispatch + Sync + Send + 'static>>,
    pub guard: Vec<Box<dyn MailGuard + Sync + Send + 'static>>,
    pub esmtp: Vec<Box<dyn EsmtpService + Sync + Send + 'static>>,
}

impl Builder {
    /// Use a given MailSetup to build a MailService.
    ///
    /// See MailSetup for examples.
    pub fn using(mut self, setup: impl MailSetup) -> Self {
        trace!("Service builder {} using setup {:?}", self.config.logging_id, setup);
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
            tls: Box::new(NoTls),
            interpretter: Arc::new(Dummy),
            dispatch: Default::default(),
            guard: Default::default(),
            esmtp: Default::default(),
        }
    }
}
