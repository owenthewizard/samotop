mod body;
mod data;
mod helo;
mod invalid;
mod mail;
mod noop;
mod quit;
mod rcpt;
mod rset;
mod session;
mod unknown;

pub use self::body::*;
pub use self::data::*;
pub use self::helo::*;
pub use self::invalid::*;
pub use self::mail::*;
pub use self::noop::*;
pub use self::quit::*;
pub use self::rcpt::*;
pub use self::rset::*;
pub use self::session::*;
pub use self::unknown::*;

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
