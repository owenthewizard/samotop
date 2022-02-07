mod body;
mod data;
mod helo;
mod invalid;
mod mail;
mod noop;
mod quit;
mod rcpt;
mod rset;
mod unknown;

pub(crate) use self::body::apply_mail_body;
pub(crate) use self::helo::apply_helo;
use crate::builder::{ServerContext, Setup};
use crate::common::*;
use crate::io::{ConnectionInfo, Handler, HandlerService};
use crate::smtp::command::*;
use crate::smtp::*;
use async_std::io::WriteExt;

/// An implementation of ESMTP - RFC 5321 - Simple Mail Transfer Protocol

#[derive(Eq, PartialEq, Debug, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Esmtp;

pub type Rfc5321 = Esmtp;

impl Setup for Esmtp {
    fn setup(&self, builder: &mut ServerContext) {
        builder.store.add::<HandlerService>(Arc::new(self.clone()));
    }
}

impl Handler for Esmtp {
    fn handle<'s, 'a, 'f>(
        &'s self,
        session: &'a mut crate::server::Session,
    ) -> S2Fut<'f, Result<()>>
    where
        's: 'f,
        'a: 'f,
    {
        session.store.add::<InterptetService>(Arc::new(
            Interpretter::apply(Esmtp)
                .to::<SmtpCommand>()
                .to::<MailBody<Vec<u8>>>()
                .build(),
        ));
        let name = session
            .store
            .get_ref::<ConnectionInfo>()
            .map(|c| c.service_name.clone())
            .unwrap_or_else(|| "samotop".to_owned());

        Box::pin(async move {
            session
                .io
                .write_all(format!("220 {} service ready\r\n", name).as_bytes())
                .await?;
            Ok(())
        })
    }
}

impl Action<SmtpCommand> for Esmtp {
    fn apply<'a, 's, 'f>(&'a self, cmd: SmtpCommand, state: &'s mut SmtpContext) -> S2Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
            use SmtpCommand as C;
            match cmd {
                C::Helo(helo) => self.apply(helo, state).await,
                C::Mail(mail) => self.apply(mail, state).await,
                C::Rcpt(rcpt) => self.apply(rcpt, state).await,
                C::Data => self.apply(SmtpData, state).await,
                C::Quit => self.apply(SmtpQuit, state).await,
                C::Rset => self.apply(SmtpRset, state).await,
                C::Noop(_) => self.apply(SmtpNoop, state).await,
                C::Expn(_) | C::Vrfy(_) | C::Help(_) | C::Turn | C::Other(_, _) => {
                    self.apply(SmtpUnknownCommand::default(), state).await
                }
            };
        })
    }
}
