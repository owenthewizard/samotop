mod body;
mod helo;

use crate::common::S1Fut;
use crate::mail::Banner;
use crate::mail::Esmtp;
use crate::smtp::command::MailBody;
use crate::smtp::command::SmtpCommand;
use crate::smtp::*;

/// An implementation of LMTP - RFC 2033 - Local Mail Transfer Protocol
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct Lmtp;

pub type Rfc2033 = Lmtp;

impl Lmtp {
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
            .and_apply(Lmtp)
            .parse::<MailBody<Vec<u8>>>()
            .with(parser)
            .and_apply(Lmtp)
    }
}

impl Action<SmtpCommand> for Lmtp {
    fn apply<'a, 's, 'f>(&'a self, cmd: SmtpCommand, state: &'s mut SmtpState) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
            use SmtpCommand as C;
            match cmd {
                C::Helo(helo) => Lmtp.apply(helo, state).await,
                cmd => Esmtp.apply(cmd, state).await,
            }
        })
    }
}
