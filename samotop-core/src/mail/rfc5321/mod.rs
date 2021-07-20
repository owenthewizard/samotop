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
use crate::common::{ready, S1Fut};
use crate::smtp::command::*;
use crate::smtp::*;

/// An implementation of ESMTP - RFC 5321 - Simple Mail Transfer Protocol

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct Esmtp;

pub type Rfc5321 = Esmtp;

impl Esmtp {
    pub fn with<P>(&self, parser: P) -> Interpretter
    where
        P: Send + Sync + 'static,
        P: Clone,
        P: Parser<SmtpCommand>,
        P: Parser<MailBody<Vec<u8>>>,
    {
        Interpretter::default()
            .call(Banner)
            .parse::<SmtpCommand>()
            .with(parser.clone())
            .and_apply(Esmtp)
            .parse::<MailBody<Vec<u8>>>()
            .with(parser)
            .and_apply(Esmtp)
    }
}

#[derive(Debug)]
pub struct Banner;

impl Interpret for Banner {
    fn interpret<'a, 'i, 's, 'f>(
        &'a self,
        _input: &'i [u8],
        state: &'s mut SmtpState,
    ) -> S1Fut<'f, InterpretResult>
    where
        'a: 'f,
        'i: 'f,
        's: 'f,
    {
        Box::pin(ready(if !state.session.banner_sent {
            state.say_service_ready();
            state.session.banner_sent = true;
            Ok(None)
        } else {
            Err(ParseError::Mismatch("Banner has already been sent".into()))
        }))
    }
}

impl Action<SmtpCommand> for Esmtp {
    fn apply<'a, 's, 'f>(&'a self, cmd: SmtpCommand, state: &'s mut SmtpState) -> S1Fut<'f, ()>
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
