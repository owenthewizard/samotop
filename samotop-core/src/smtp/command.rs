pub use super::commands::*;
use super::SmtpReply;
use crate::{common::*, smtp::state::SmtpState};
use std::fmt;

pub trait SmtpSessionCommand: Sync + Send + fmt::Debug {
    fn verb(&self) -> &str;
    #[must_use = "apply must be awaited"]
    fn apply(&self, state: SmtpState) -> S2Fut<SmtpState>;
}

pub trait ApplyCommand<Data> {
    #[must_use = "apply must be awaited"]
    fn apply_cmd(data: &Data, state: SmtpState) -> S2Fut<SmtpState>;
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum SmtpCommand {
    StartTls,
    Helo(SmtpHelo),
    Mail(SmtpMail),
    Rcpt(SmtpRcpt),
    Expn(String),
    Vrfy(String),
    Help(Vec<String>),
    Noop(Vec<String>),
    Quit,
    Rset,
    Data,
    Turn,
    /// Command outside of the base implementation.
    /// First string is the command verb, next the parameters
    Other(String, Vec<String>),
}

impl SmtpCommand {
    pub fn verb(&self) -> &str {
        use SmtpCommand as C;
        match self {
            C::Helo(ref helo) => helo.verb.as_ref(),
            C::Mail(ref mail) => mail.verb(),
            C::Rcpt(_) => "RCPT",
            C::Data => "DATA",
            C::Quit => "QUIT",
            C::Rset => "RSET",
            C::Noop(_) => "NOOP",
            C::StartTls => "STARTTLS",
            C::Expn(_) => "EXPN",
            C::Vrfy(_) => "VRFY",
            C::Help(_) => "HELP",
            C::Turn => "TURN",
            C::Other(ref verb, _) => verb.as_str(),
        }
    }
}

impl<T, E> SmtpSessionCommand for std::result::Result<T, E>
where
    T: SmtpSessionCommand,
    E: fmt::Debug + Sync + Send,
{
    fn verb(&self) -> &str {
        ""
    }

    fn apply(&self, mut state: SmtpState) -> S2Fut<SmtpState> {
        match self {
            Ok(command) => Box::pin(async move { command.apply(state).await }),
            Err(e) => {
                error!("reading SMTP input failed: {:?}", e);
                state.say_shutdown(SmtpReply::ProcesingError);
                Box::pin(ready(state))
            }
        }
    }
}
