mod body;
mod helo;

use crate::{
    common::*,
    io::tls::MayBeTls,
    mail::{Configuration, MailSetup},
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

impl MailSetup for Lmtp {
    fn setup(self, config: &mut Configuration) {
        config.add_last_interpretter(
            Interpretter::apply(Lmtp)
                .to::<SmtpCommand>()
                //.parse::<MailBody<Vec<u8>>>()
                .to::<MailBody<Vec<u8>>>()
                .build(),
        );
        config.add_last_session_service(self);
    }
}

impl SessionService for Lmtp {
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
        Esmtp.prepare_session(io, state)
    }
}
