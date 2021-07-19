use crate::{
    common::*,
    io::tls::{NoTls, TlsProvider},
    mail::{EsmtpService, MailDispatch, MailGuard, MailSetup, Service},
    smtp::{Dummy, Interpret},
};

#[derive(Default)]
pub struct Builder {
    config: Configuration,
}

#[derive(Debug)]
pub struct Configuration {
    pub id: String,
    pub tls: Box<dyn TlsProvider + Sync + Send + 'static>,
    pub interpretter: Arc<dyn Interpret + Send + Sync>,
    pub dispatch: Vec<Box<dyn MailDispatch + Sync + Send + 'static>>,
    pub guard: Vec<Box<dyn MailGuard + Sync + Send + 'static>>,
    pub esmtp: Vec<Box<dyn EsmtpService + Sync + Send + 'static>>,
}

impl Builder {
    pub fn using(mut self, setup: impl MailSetup) -> Self {
        trace!("Builder {} using setup {:?}", self.config.id, setup);
        setup.setup(&mut self.config);
        self
    }
    pub fn build(self) -> Service {
        Service::new(self.config)
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            id: Default::default(),
            tls: Box::new(NoTls),
            interpretter: Arc::new(Dummy),
            dispatch: Default::default(),
            guard: Default::default(),
            esmtp: Default::default(),
        }
    }
}
