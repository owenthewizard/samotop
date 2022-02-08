#[macro_use]
extern crate log;
mod data;
mod smtp;

pub use self::data::*;
pub use self::smtp::*;
use samotop_core::io::Session;
pub use samotop_core::smtp::{ParseError, ParseResult, Parser};
use samotop_core::{
    config::{ServerContext, Setup},
    common::*,
    io::{Handler, HandlerService},
    smtp::{
        command::{MailBody, SmtpCommand},
        ParserService, StartTls,
    },
};

#[derive(Clone, Copy, Debug, Default, )]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct SmtpParserPeg;

impl Setup for SmtpParserPeg {
    fn setup(&self, ctx: &mut ServerContext) {
        ctx.store.add::<HandlerService>(Arc::new(SmtpParserPeg))
    }
}

impl Handler for SmtpParserPeg {
    fn handle<'s, 'a, 'f>(&'s self, session: &'a mut Session) -> S2Fut<'f, Result<()>>
    where
        's: 'f,
        'a: 'f,
    {
        session
            .store
            .set::<ParserService<SmtpCommand>>(Box::new(SmtpParserPeg));
        session
            .store
            .set::<ParserService<StartTls>>(Box::new(SmtpParserPeg));
        session
            .store
            .set::<ParserService<MailBody<Vec<u8>>>>(Box::new(SmtpParserPeg));
        Box::pin(ready(Ok(())))
    }
}
