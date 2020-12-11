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

use super::rfc3207::ESMTPStartTls;
use crate::common::*;
use crate::smtp::*;

/// An implementation of ESMTP - RFC 5321 - SMTP Service Extension for Secure SMTP over Transport Layer Security

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct ESMTP;

pub type Rfc5321 = ESMTP;

impl Rfc5321 {
    pub fn command<I>(instruction: I) -> ESMTPCommand<I> {
        ESMTPCommand { instruction }
    }
}

impl ApplyCommand<SmtpCommand> for Rfc5321 {
    fn apply_cmd(cmd: &SmtpCommand, state: SmtpState) -> S2Fut<SmtpState> {
        use SmtpCommand as C;
        Box::pin(async move {
            match cmd {
                C::Helo(ref helo) => Self::apply_cmd(helo, state).await,
                C::Mail(ref mail) => Self::apply_cmd(mail, state).await,
                C::Rcpt(ref rcpt) => Self::apply_cmd(rcpt, state).await,
                C::Data => Self::apply_cmd(&SmtpData, state).await,
                C::Quit => Self::apply_cmd(&SmtpQuit, state).await,
                C::Rset => Self::apply_cmd(&SmtpRset, state).await,
                C::Noop(_) => Self::apply_cmd(&SmtpNoop, state).await,
                C::StartTls => ESMTPStartTls::command().apply(state).await,
                C::Expn(_) | C::Vrfy(_) | C::Help(_) | C::Turn | C::Other(_, _) => {
                    Self::apply_cmd(&SmtpUnknownCommand::default(), state).await
                }
            }
        })
    }
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct ESMTPCommand<I> {
    instruction: I,
}

impl SmtpSessionCommand for ESMTPCommand<SmtpCommand> {
    fn verb(&self) -> &str {
        self.instruction.verb()
    }

    fn apply(&self, state: SmtpState) -> S2Fut<SmtpState> {
        ESMTP::apply_cmd(&self.instruction, state)
    }
}
