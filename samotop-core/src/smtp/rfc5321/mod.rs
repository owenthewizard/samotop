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
use crate::common::*;
use crate::io::tls::MayBeTls;
use crate::mail::{Configuration, MailSetup};
use crate::smtp::command::*;
use crate::smtp::*;

/// An implementation of ESMTP - RFC 5321 - Simple Mail Transfer Protocol

#[derive(Eq, PartialEq, Debug, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Esmtp;

pub type Rfc5321 = Esmtp;

impl MailSetup for Esmtp {
    fn setup(self, config: &mut Configuration) {
        config.add_last_interpretter(
            Interpretter::apply(Esmtp)
                .to::<SmtpCommand>()
                .to::<MailBody<Vec<u8>>>()
                .build(),
        );
        config.add_last_session_service(self);
    }
}

impl SessionService for Esmtp {
    fn prepare_session<'a, 'i, 's, 'f>(
        &'a self,
        _io: &'i mut Box<dyn MayBeTls>,
        state: &'s mut SmtpContext,
    ) -> S1Fut<'f, ()>
    where
        'a: 'f,
        'i: 'f,
        's: 'f,
    {
        state.session.say_service_ready();
        Box::pin(ready(()))
    }
}

impl Action<SmtpCommand> for Esmtp {
    fn apply<'a, 's, 'f>(&'a self, cmd: SmtpCommand, state: &'s mut SmtpContext) -> S1Fut<'f, ()>
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
