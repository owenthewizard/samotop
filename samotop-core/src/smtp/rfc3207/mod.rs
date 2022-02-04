use crate::common::{ready, S1Fut};
use crate::io::tls::{MayBeTls, TlsProvider};
use crate::mail::{Configuration, MailSetup};
use crate::smtp::{extension, Interpretter, SessionService, SmtpContext};

mod starttls;

/// An implementation of ESMTP STARTTLS - RFC 3207 - SMTP Service Extension for Secure SMTP over Transport Layer Security
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct StartTls;

pub type Rfc3207 = StartTls;

impl MailSetup for StartTls {
    fn setup(self, config: &mut Configuration) {
        config.add_last_interpretter(Interpretter::apply(StartTls).to::<StartTls>().build());
        config.add_last_session_service(self);
    }
}

pub type TlsService = Box<dyn TlsProvider + Send + Sync>;

impl SessionService for StartTls {
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
                if let Some(tls) = state.get::<TlsService>() {
                    if let Some(upgrade) = tls.get_tls_upgrade() {
                        io.enable_encryption(upgrade, String::default());
                    }
                } else {
                    warn!("No TLS provider")
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
