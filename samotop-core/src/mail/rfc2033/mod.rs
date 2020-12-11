mod body;
mod helo;

use super::Rfc5321;
use crate::common::*;
use crate::smtp::*;

/// An implementation of LMTP - RFC 2033 - Local Mail Transfer Protocol
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct LMTP<I> {
    instruction: I,
}
pub type Rfc2033<I> = LMTP<I>;

impl<I> Rfc2033<I> {
    pub fn new(instruction: I) -> Self {
        Self { instruction }
    }
}

impl SmtpSessionCommand for Rfc2033<SmtpCommand> {
    fn verb(&self) -> &str {
        self.instruction.verb()
    }

    fn apply(&self, state: SmtpState) -> S2Fut<SmtpState> {
        use SmtpCommand as C;
        Box::pin(async move {
            match self.instruction {
                C::Helo(ref helo) => Self::apply_helo(helo, state).await,
                C::Mail(ref mail) => Rfc5321::<()>::apply_mail(mail, state).await,
                C::Rcpt(ref rcpt) => rcpt.apply(state).await,
                C::Data => Rfc5321::<()>::apply_data(state).await,
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
