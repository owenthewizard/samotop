#[macro_use]
extern crate log;
mod data;
mod smtp;

pub use self::data::*;
pub use self::smtp::*;
pub use samotop_core::smtp::{ParseError, ParseResult, Parser};
use samotop_core::{
    common::S1Fut,
    io::tls::MayBeTls,
    mail::{Configuration, MailSetup},
    smtp::{
        command::{MailBody, SmtpCommand},
        ParserService, SessionSetup, SmtpContext, StartTls,
    },
};
use serde::{Deserialize, Serialize};
use std::future::ready;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct SmtpParserPeg;

impl MailSetup for SmtpParserPeg {
    fn setup(self, config: &mut Configuration) {
        config.add_last_session_service(SmtpParserPeg)
    }
}

impl SessionSetup for SmtpParserPeg {
    fn setup_session<'a, 'i, 's, 'f>(
        &'a self,
        _io: &'i mut Box<dyn MayBeTls>,
        state: &'s mut SmtpContext,
    ) -> S1Fut<'f, ()>
    where
        'a: 'f,
        'i: 'f,
        's: 'f,
    {
        state.store.set::<ParserService<SmtpCommand>>(Box::new(SmtpParserPeg));
        state.store.set::<ParserService<StartTls>>(Box::new(SmtpParserPeg));
        state.store.set::<ParserService<MailBody<Vec<u8>>>>(Box::new(SmtpParserPeg));
        Box::pin(ready(()))
    }
}
