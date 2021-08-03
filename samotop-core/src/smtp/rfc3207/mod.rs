use crate::common::{ready, S1Fut};
use crate::io::tls::{MayBeTls, TlsProvider};
use crate::mail::{AcceptsInterpretter, AcceptsSessionService, MailSetup};
use crate::smtp::{extension, Interpretter, Parser, SessionService, SmtpContext};
use std::sync::Arc;

mod starttls;

/// An implementation of ESMTP STARTTLS - RFC 3207 - SMTP Service Extension for Secure SMTP over Transport Layer Security
#[derive(Debug)]
pub struct EsmtpStartTls;

pub type Rfc3207 = EsmtpStartTls;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartTls;

#[derive(Debug)]
pub struct EsmtpStartTlsConfigured {
    tls: Box<dyn TlsProvider + Sync + Send + 'static>,
    interpret: Arc<Interpretter>,
}

impl EsmtpStartTls {
    pub fn with<
        P: Parser<StartTls> + Send + Sync + 'static,
        TLS: TlsProvider + Sync + Send + 'static,
    >(
        &self,
        parser: P,
        provider: TLS,
    ) -> EsmtpStartTlsConfigured {
        EsmtpStartTlsConfigured {
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

impl<T: AcceptsSessionService + AcceptsInterpretter> MailSetup<T> for EsmtpStartTlsConfigured {
    fn setup(self, config: &mut T) {
        config.add_last_interpretter(self.interpret.clone());
        config.add_last_session_service(self);
    }
}

impl SessionService for EsmtpStartTlsConfigured {
    fn prepare_session<'a, 'i, 's, 'f>(
        &'a self,
        io: &'i mut Box<dyn MayBeTls>,
        state: &'s mut SmtpContext,
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
