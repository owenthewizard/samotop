#[macro_use]
extern crate log;
mod data;
mod smtp;

pub use self::data::*;
pub use self::smtp::*;
use samotop_core::builder::ServerContext;
use samotop_core::builder::Setup;
use samotop_core::io::Handler;
use samotop_core::io::HandlerService;
use samotop_core::smtp::{
    command::{MailBody, SmtpCommand},
    ParserService, StartTls,
};
pub use samotop_core::smtp::{ParseError, ParseResult, Parser};
use serde::{Deserialize, Serialize};
use std::future::ready;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct SmtpParserPeg;

impl Setup for SmtpParserPeg {
    fn setup(&self, ctx: &mut ServerContext) {
        ctx.store.add::<HandlerService>(Arc::new(SmtpParserPeg))
    }
}

impl Handler for SmtpParserPeg {
    fn handle<'s, 'a, 'f>(
        &'s self,
        session: &'a mut samotop_core::server::Session,
    ) -> samotop_core::common::S2Fut<'f, samotop_core::common::Result<()>>
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
