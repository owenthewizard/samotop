mod body;
mod helo;

use super::Rfc5321;
use crate::common::*;
use crate::smtp::*;

/// An implementation of LMTP - RFC 2033 - Local Mail Transfer Protocol
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct LMTP;

pub type Rfc2033 = LMTP;

impl Rfc2033 {
    pub fn new<I>(instruction: I) -> LMTPCommand<I> {
        LMTPCommand { instruction }
    }
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct LMTPCommand<I> {
    instruction: I,
}

impl SmtpSessionCommand for LMTPCommand<SmtpCommand> {
    fn verb(&self) -> &str {
        self.instruction.verb()
    }

    fn apply(&self, state: SmtpState) -> S2Fut<SmtpState> {
        use SmtpCommand as C;
        Box::pin(async move {
            match self.instruction {
                C::Helo(ref helo) => Rfc2033::apply_cmd(helo, state).await,
                ref cmd => Rfc5321::apply_cmd(cmd, state).await,
            }
        })
    }
}
