use crate::common::{ready, S1Fut};
use crate::io::tls::{MayBeTls, TlsProvider};
use crate::mail::{Configuration, MailSetup};
use crate::smtp::{extension, EsmtpService, Interpretter, Parser, SmtpState};
use std::sync::Arc;

mod starttls;

/// An implementation of ESMTP STARTTLS - RFC 3207 - SMTP Service Extension for Secure SMTP over Transport Layer Security
#[derive(Debug)]
pub struct EsmtpStartTls {
    tls: Box<dyn TlsProvider + Sync + Send + 'static>,
    interpret: Arc<Interpretter>,
}

pub type Rfc3207 = EsmtpStartTls;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartTls;

impl EsmtpStartTls {
    pub fn with<
        P: Parser<StartTls> + Send + Sync + 'static,
        TLS: TlsProvider + Sync + Send + 'static,
    >(
        parser: P,
        provider: TLS,
    ) -> Self {
        Self {
            tls: Box::new(provider),
            interpret: Arc::new(
                Interpretter::default()
                    .parse::<StartTls>()
                    .with(parser)
                    .and_apply(StartTls),
            ),
        }
    }
}

impl MailSetup for EsmtpStartTls {
    fn setup(self, config: &mut Configuration) {
        config.interpret.insert(0, Box::new(self.interpret.clone()));
        config.esmtp.insert(0, Box::new(self));
    }
}

impl EsmtpService for EsmtpStartTls {
    fn prepare_session<'a, 'i, 's, 'f>(
        &'a self,
        io: &'i mut Box<dyn MayBeTls>,
        state: &'s mut SmtpState,
    ) -> S1Fut<'f, ()>
    where
        'a: 'f,
        'i: 'f,
        's: 'f,
    {
        if !io.is_encrypted() {
            // Add tls if needed and available
            if !io.can_encrypt() {
                if let Some(upgrade) = self.tls.get_tls_upgrade() {
                    io.enable_encryption(upgrade, String::default());
                }
            }
            // enable STARTTLS extension if it can be used
            if io.can_encrypt() {
                state.session.extensions.enable(&extension::STARTTLS);
            }
        }
        Box::pin(ready(()))
    }
}
