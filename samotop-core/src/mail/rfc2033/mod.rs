mod body;
mod helo;

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
            .parse::<SmtpCommand>()
            .with(parser.clone())
            .and_apply(Lmtp)
            .parse::<MailBody<Vec<u8>>>()
            .with(parser)
            .and_apply(Lmtp)
    }
}

#[async_trait::async_trait]
impl Action<SmtpCommand> for Lmtp {
    async fn apply(&self, cmd: SmtpCommand, state: &mut SmtpState) {
        use SmtpCommand as C;
        match cmd {
            C::Helo(helo) => Lmtp.apply(helo, state).await,
            cmd => Esmtp.apply(cmd, state).await,
        }
    }
}
