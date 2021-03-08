mod body;
mod helo;

use super::Rfc5321;
use crate::common::*;
use crate::smtp::*;

/// An implementation of LMTP - RFC 2033 - Local Mail Transfer Protocol
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct Lmtp;

pub type Rfc2033 = Lmtp;

impl Rfc2033 {
    pub fn command<I>(instruction: I) -> LmtpCommand<I> {
        LmtpCommand { instruction }
    }
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct LmtpCommand<I> {
    instruction: I,
}

impl SmtpSessionCommand for LmtpCommand<SmtpCommand> {
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
