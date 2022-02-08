mod body;
mod data;
mod extensions;
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
use crate::config::{ServerContext, Setup};
use crate::common::*;
use crate::io::{ConnectionInfo, Handler, HandlerService, Session};
use crate::smtp::command::*;
use crate::smtp::*;

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
    fn handle<'s, 'a, 'f>(&'s self, session: &'a mut Session) -> S2Fut<'f, Result<()>>
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
        let mut local_name = session
            .store
            .get_ref::<ConnectionInfo>()
            .map(|c| c.local_addr.clone())
            .unwrap_or_else(|| "samotop".to_owned());

        let smtp = session.store.get_or_compose::<SmtpSession>();

        if !smtp.service_name.is_empty() {
            local_name = smtp.service_name.clone();
        }

        smtp.say_service_ready(local_name);

        Box::pin(ready(Ok(())))
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
