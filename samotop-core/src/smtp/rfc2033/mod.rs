mod body;
mod helo;

use crate::{
    builder::{ServerContext, Setup},
    common::*,
    io::{Handler, HandlerService},
    smtp::{
        command::{MailBody, SmtpCommand},
        *,
    },
};

/// An implementation of LMTP - RFC 2033 - Local Mail Transfer Protocol
#[derive(Eq, PartialEq, Debug, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Lmtp;

pub type Rfc2033 = Lmtp;

impl Setup for Lmtp {
    fn setup(&self, builder: &mut ServerContext) {
        builder.store.add::<HandlerService>(Arc::new(self.clone()));
    }
}

impl Handler for Lmtp {
    fn handle<'s, 'a, 'f>(
        &'s self,
        session: &'a mut crate::server::Session,
    ) -> S2Fut<'f, Result<()>>
    where
        's: 'f,
        'a: 'f,
    {
        Box::pin(async move {
            Esmtp.handle(session).await?;

            session.store.add::<InterptetService>(Arc::new(
                Interpretter::apply(Lmtp)
                    .to::<SmtpCommand>()
                    //.parse::<MailBody<Vec<u8>>>()
                    .to::<MailBody<Vec<u8>>>()
                    .build(),
            ));

            Ok(())
        })
    }
}
