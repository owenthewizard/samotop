use crate::io::tls::TlsProvider;
use crate::mail::{EsmtpService, MailSetup};
use crate::smtp::{extension, Interpretter, Parser};
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
    fn setup(self, config: &mut crate::mail::Configuration) {
        config.interpret.insert(0, Box::new(self.interpret.clone()));
        config.esmtp.insert(0, Box::new(self));
    }
}

impl EsmtpService for EsmtpStartTls {
    fn prepare_session(
        &self,
        io: &mut dyn crate::io::tls::MayBeTls,
        session: &mut crate::mail::SessionInfo,
    ) {
        if !io.is_encrypted() {
            // Add tls if needed and available
            if !io.can_encrypt() {
                if let Some(upgrade) = self.tls.get_tls_upgrade() {
                    io.enable_encryption(upgrade, String::default());
                }
            }
            // enable STARTTLS extension if it can be used
            if io.can_encrypt() {
                session.extensions.enable(&extension::STARTTLS);
            }
        }
    }
}
