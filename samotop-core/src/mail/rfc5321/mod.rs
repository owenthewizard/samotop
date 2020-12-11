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

use crate::common::*;
use crate::smtp::*;

/// An implementation of ESMTP - RFC 5321 - SMTP Service Extension for Secure SMTP over Transport Layer Security

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct ESMTP<I> {
    instruction: I,
}

pub type Rfc5321<I> = ESMTP<I>;

impl<I> Rfc5321<I> {
    pub fn new(instruction: I) -> Self {
        Self { instruction }
    }
}

impl SmtpSessionCommand for Rfc5321<SmtpCommand> {
    fn verb(&self) -> &str {
        self.instruction.verb()
    }

    fn apply(&self, state: SmtpState) -> S2Fut<SmtpState> {
        use SmtpCommand as C;
        Box::pin(async move {
            match self.instruction {
                C::Helo(ref helo) => Self::apply_helo(helo, state).await,
                C::Mail(ref mail) => Self::apply_mail(mail, state).await,
                C::Rcpt(ref rcpt) => rcpt.apply(state).await,
                C::Data => Self::apply_data(state).await,
                C::Quit => SmtpQuit.apply(state).await,
                C::Rset => SmtpRset.apply(state).await,
                C::Noop(_) => SmtpNoop.apply(state).await,
                C::StartTls => StartTls.apply(state).await,
                C::Expn(_) | C::Vrfy(_) | C::Help(_) | C::Turn | C::Other(_, _) => {
                    SmtpUnknownCommand::default().apply(state).await
                }
            }
        })
    }
}
