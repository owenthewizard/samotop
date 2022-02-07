use std::sync::Arc;

use crate::builder::{ServerContext, Setup};
use crate::common::ready;
use crate::io::tls::TlsProvider;
use crate::io::{ConnectionInfo, Handler, HandlerService};
use crate::smtp::{extension, Interpretter};
use crate::store::{Component, SingleComponent};

use super::{InterptetService, SmtpSession};

mod starttls;

/// An implementation of ESMTP STARTTLS - RFC 3207 - SMTP Service Extension for Secure SMTP over Transport Layer Security
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct StartTls;

pub type Rfc3207 = StartTls;

impl Setup for StartTls {
    fn setup(&self, ctx: &mut ServerContext) {
        ctx.store.add::<HandlerService>(Arc::new(self.clone()));
    }
}

pub struct TlsService {}
impl Component for TlsService {
    type Target = Arc<dyn TlsProvider + Send + Sync>;
}
impl SingleComponent for TlsService {}

impl Handler for StartTls {
    fn handle<'s, 'a, 'f>(
        &'s self,
        session: &'a mut crate::server::Session,
    ) -> crate::common::S2Fut<'f, crate::common::Result<()>>
    where
        's: 'f,
        'a: 'f,
    {
        session.store.add::<InterptetService>(Arc::new(
            Interpretter::apply(StartTls).to::<StartTls>().build(),
        ));

        let is_encrypted = session
            .store
            .get_ref::<ConnectionInfo>()
            .map(|c| c.encrypted)
            .unwrap_or_default();

        if !is_encrypted {
            // Add tls if needed and available
            if session.store.get_ref::<TlsService>().is_some() {
                session
                    .store
                    .get_or_compose::<SmtpSession>()
                    .extensions
                    .enable(&extension::STARTTLS);
            } else {
                warn!("No TLS provider")
            }
        }
        Box::pin(ready(Ok(())))
    }
}
