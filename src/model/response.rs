/* codes are arranged according to rfc5321 + rfc7504:

   2yz  Positive Completion reply
      The requested action has been successfully completed.  A new
      request may be initiated.

   3yz  Positive Intermediate reply
      The command has been accepted, but the requested action is being
      held in abeyance, pending receipt of further information.  The
      SMTP client should send another command specifying this
      information.  This reply is used in command sequence groups (i.e.,
      in DATA).

   4yz  Transient Negative Completion reply
      The command was not accepted, and the requested action did not
      occur.  However, the error condition is temporary, and the action
      may be requested again.  The sender should return to the beginning
      of the command sequence (if any).  It is difficult to assign a
      meaning to "transient" when two different sites (receiver- and
      sender-SMTP agents) must agree on the interpretation.  Each reply
      in this category might have a different time value, but the SMTP
      client SHOULD try again.  A rule of thumb to determine whether a
      reply fits into the 4yz or the 5yz category (see below) is that
      replies are 4yz if they can be successful if repeated without any
      change in command form or in properties of the sender or receiver
      (that is, the command is repeated identically and the receiver
      does not put up a new implementation).

   5yz  Permanent Negative Completion reply
      The command was not accepted and the requested action did not
      occur.  The SMTP client SHOULD NOT repeat the exact request (in
      the same sequence).  Even some "permanent" error conditions can be
      corrected, so the human user may want to direct the SMTP client to
      reinitiate the command sequence by direct action at some point in
      the future (e.g., after the spelling has been changed, or the user
      has altered the account status).

   x0z  Syntax: These replies refer to syntax errors, syntactically
      correct commands that do not fit any functional category, and
      unimplemented or superfluous commands.

   x1z  Information: These are replies to requests for information, such
      as status or help.

   x2z  Connections: These are replies referring to the transmission
      channel.

   x3z  Unspecified.

   x4z  Unspecified.

   x5z  Mail system: These replies indicate the status of the receiver
      mail system vis-a-vis the requested transfer or other mail system
      action.
*/
use std::fmt;
use model::response::SmtpReply::*;

#[derive(Eq, PartialEq, Debug)]
pub enum SmtpReply {
    // I'm using a suffix to make names sound english:
    // 2xx => ...Info
    // 3xx => ...Challenge
    // 4xx => ...Error
    // 5xx => ...Failure

    /* no response should be given */
    None,

    /*500*/
    CommandSyntaxFailure,
    /*501*/
    ParameterSyntaxFailure,
    /*502*/
    CommandNotImplementedFailure,
    /*503*/
    CommandSequenceFailure,
    /*504*/
    UnexpectedParameterFailure,

    /*211*/
    StatusInfo(String),
    /*214*/
    HelpInfo(String),

    // 220 <domain> Service ready
    ServiceReadyInfo(String),
    // 221 <domain> Service closing transmission channel
    ClosingConnectionInfo(String),
    // 421 <domain> Service not available, closing transmission channel
    ServiceNotAvailableError(String),
    // 521 RFC 7504
    MailNotAcceptedByHostFailure,

    // 250 first line is either Ok or specific message, use Vec<String> for subsequent items
    OkInfo,
    OkMessageInfo(String),
    OkHeloInfo { local: String, remote: String },
    OkEhloInfo {
        local: String,
        remote: String,
        extensions: Vec<SmtpExtension>,
    },
    // 251 will forward to <forward-path> (See Section 3.4)
    UserNotLocalInfo(String),
    // 252 but will accept message and attempt delivery (See Section 3.5.3)
    CannotVerifyUserInfo,
    // 354 end with <CRLF>.<CRLF>
    StartMailInputChallenge,
    // 450 Requested mail action not taken (e.g., mailbox busy
    //     or temporarily blocked for policy reasons)
    MailboxNotAvailableError,
    // 451 Requested action aborted
    ProcesingError,
    // 452 Requested action not taken
    StorageError,
    // 455 right now the parameters given cannot be accomodated
    ParametersNotAccommodatedError,
    // 550 Requested action not taken: mailbox unavailable (e.g.,
    //     mailbox not found, no access, or command rejected for policy reasons)
    MailboxNotAvailableFailure,
    // 551 please try <forward-path> (See Section 3.4)
    UserNotLocalFailure(String),
    // 552 Requested mail action aborted: exceeded storage allocation
    StorageFailure,
    // 553 Requested action not taken: mailbox name not allowed (e.g., mailbox syntax incorrect)
    MailboxNameInvalidFailure,
    // 554 (Or, in the case of a connection-opening response, "No SMTP service here")
    TransactionFailure,
    // 555 MAIL FROM/RCPT TO parameters not recognized or not implemented
    UnknownMailParametersFailure,
    // 556 RFC 7504
    MailNotAcceptedByDomainFailure,
}

impl SmtpReply {
    pub fn code(&self) -> u16 {
        match self {
            /* &Custom(ref class, ref category, ref digit, _, _) => {
                *class as u16 + *category as u16 + *digit as u16
            }*/
            &None => 0,
            &CommandSyntaxFailure => 500,
            &ParameterSyntaxFailure => 501,
            &CommandNotImplementedFailure => 502,
            &CommandSequenceFailure => 503,
            &UnexpectedParameterFailure => 504,

            &StatusInfo(_) => 211,
            &HelpInfo(_) => 214,

            // <domain> Service ready
            &ServiceReadyInfo(_) => 220,
            // <domain> Service closing transmission channel
            &ClosingConnectionInfo(_) => 221,
            // <domain> Service not available, closing transmission channel
            &ServiceNotAvailableError(_) => 421,
            // RFC 7504
            &MailNotAcceptedByHostFailure => 521,

            // first line is either Ok or specific message, use Vec<String> for subsequent items
            &OkInfo => 250,
            &OkMessageInfo(_) => 250,
            &OkHeloInfo { .. } => 250,
            &OkEhloInfo { .. } => 250,
            // will forward to <forward-path> (See Section 3.4)
            &UserNotLocalInfo(_) => 251,
            //, but will accept message and attempt delivery (See Section 3.5.3)
            &CannotVerifyUserInfo => 252,
            // end with <CRLF>.<CRLF>
            &StartMailInputChallenge => 354,
            // Requested mail action not taken (e.g., mailbox busy
            // or temporarily blocked for policy reasons)
            &MailboxNotAvailableError => 450,
            // Requested action aborted
            &ProcesingError => 451,
            // Requested action not taken
            &StorageError => 452,
            // right now the parameters given cannot be accomodated
            &ParametersNotAccommodatedError => 455,
            // Requested action not taken: mailbox unavailable (e.g.,
            // mailbox not found, no access, or command rejected for policy reasons)
            &MailboxNotAvailableFailure => 550,
            // please try <forward-path> (See Section 3.4)
            &UserNotLocalFailure(_) => 551,
            // Requested mail action aborted: exceeded storage allocation
            &StorageFailure => 552,
            // Requested action not taken: mailbox name not allowed (e.g., mailbox syntax incorrect)
            &MailboxNameInvalidFailure => 553,
            // (Or, in the case of a connection-opening response, "No SMTP service here")
            &TransactionFailure => 554,
            // MAIL FROM/RCPT TO parameters not recognized or not implemented
            &UnknownMailParametersFailure => 555,
            // RFC 7504
            &MailNotAcceptedByDomainFailure => 556,
        }
    }

    pub fn text(&self) -> String {
        match self {
            &None => "".to_owned(),
            &CommandSyntaxFailure => "Syntax error, command unrecognized".to_owned(),
            &ParameterSyntaxFailure => "Syntax error in parameters or arguments".to_owned(),
            &CommandNotImplementedFailure => "Command not implemented".to_owned(),
            &CommandSequenceFailure => "Bad sequence of commands".to_owned(),
            &UnexpectedParameterFailure => "Command parameter not implemented".to_owned(),

            &StatusInfo(ref text) => format!("{}", text),
            &HelpInfo(ref text) => format!("{}", text),

            &ServiceReadyInfo(ref domain) => format!("{} Service ready", domain),
            &ClosingConnectionInfo(ref domain) => {
                format!("{} Service closing transmission channel", domain)
            }
            &ServiceNotAvailableError(ref domain) => {
                format!(
                    "{} Service not available, closing transmission channel",
                    domain
                )
            }
            &MailNotAcceptedByHostFailure => "Host does not accept mail".to_owned(),

            &OkInfo => "Ok".to_owned(),
            &OkMessageInfo(ref text) => format!("{}", text),
            &OkHeloInfo {
                ref local,
                ref remote,
                ..
            } => format!("{} greets {}", local, remote),
            &OkEhloInfo {
                ref local,
                ref remote,
                ..
            } => format!("{} greets {}", local, remote),

            &UserNotLocalInfo(ref forward_path) => {
                format!("User not local, will forward to {}", forward_path)
            }
            &CannotVerifyUserInfo => {
                "Cannot VFRY user, but will accept message and attempt delivery".to_owned()
            }
            &StartMailInputChallenge => "Start mail input, end with <CRLF>.<CRLF>".to_owned(),
            &MailboxNotAvailableError => {
                "Requested mail action not taken: mailbox unavailable".to_owned()
            }
            &ProcesingError => "Requested action aborted: error in processing".to_owned(),
            &StorageError => "Requested action not taken: insufficient system storage".to_owned(),
            &ParametersNotAccommodatedError => "Server unable to accommodate parameters".to_owned(),
            &MailboxNotAvailableFailure => {
                "Requested action not taken: mailbox unavailable".to_owned()
            }
            &UserNotLocalFailure(ref forward_path) => {
                format!("User not local; please try {}", forward_path)
            }
            &StorageFailure => {
                "Requested mail action aborted: exceeded storage allocation".to_owned()
            }
            &MailboxNameInvalidFailure => {
                "Requested action not taken: mailbox name not allowed".to_owned()
            }
            &TransactionFailure => "Transaction failed".to_owned(),
            &UnknownMailParametersFailure => {
                "MAIL FROM/RCPT TO parameters not recognized or not implemented".to_owned()
            }
            &MailNotAcceptedByDomainFailure => "Domain does not accept mail".to_owned(),
        }
    }
    pub fn items(&self) -> Vec<String> {
        match self {
            &OkEhloInfo { ref extensions, .. } => {
                extensions.iter().map(|e| format!("{}", e)).collect()
            }
            _ => vec![],
        }
    }
    pub fn class(&self) -> SmtpReplyClass {
        match self.code() {
            0...299 => SmtpReplyClass::Info,
            300...399 => SmtpReplyClass::Challenge,
            400...499 => SmtpReplyClass::Error,
            _ => SmtpReplyClass::Failure,
        }
    }
    pub fn category(&self) -> SmtpReplyCategory {
        match self.code() % 100 {
            0...9 => SmtpReplyCategory::Syntax,
            10...19 => SmtpReplyCategory::Information,
            20...29 => SmtpReplyCategory::Connections,
            30...39 => SmtpReplyCategory::Reserved3,
            40...49 => SmtpReplyCategory::Reserved4,
            _ => SmtpReplyCategory::System,
        }
    }
    pub fn digit(&self) -> SmtpReplyDigit {
        match self.code() % 10 {
            0 => SmtpReplyDigit::D0,
            1 => SmtpReplyDigit::D1,
            2 => SmtpReplyDigit::D2,
            3 => SmtpReplyDigit::D3,
            4 => SmtpReplyDigit::D4,
            5 => SmtpReplyDigit::D5,
            6 => SmtpReplyDigit::D6,
            7 => SmtpReplyDigit::D7,
            8 => SmtpReplyDigit::D8,
            _ => SmtpReplyDigit::D9,
        }
    }
}

impl fmt::Display for SmtpReply {
    fn fmt<'a>(&self, mut buf: &'a mut fmt::Formatter) -> Result<(), fmt::Error> {
        let code = self.code();
        let text = self.text();
        let items = self.items();

        if items.is_empty() {
            try!(write_reply_end(&mut buf, code, &text));
        } else {
            try!(write_reply_continued(&mut buf, code, &text));
            for i in 0..items.len() {
                if i == items.len() - 1 {
                    try!(write_reply_end(&mut buf, code, &items[i]));
                } else {
                    try!(write_reply_continued(&mut buf, code, &items[i]));
                }
            }
        }
        Ok(())
    }
}
fn write_reply_end(buf: &mut fmt::Write, code: u16, text: &str) -> Result<(), fmt::Error> {
    write!(buf, "{} {}", code, text)
}
fn write_reply_continued(buf: &mut fmt::Write, code: u16, text: &str) -> Result<(), fmt::Error> {
    write!(buf, "{}-{}\r\n", code, text)
}

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub enum SmtpExtension {
    EightBitMime,
    Size(usize),
}

impl fmt::Display for SmtpExtension {
    fn fmt<'a>(&self, fmt: &'a mut fmt::Formatter) -> Result<(), fmt::Error> {
        use self::SmtpExtension::*;
        match self {
            &EightBitMime => fmt.write_str("8BITMIME"),
            &Size(s) => {
                if s == 0 {
                    fmt.write_str("SIZE")
                } else {
                    fmt.write_fmt(format_args!("SIZE {}", s))
                }
            }
        }
    }
}

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub enum SmtpReplyClass {
    Info = 200,
    Challenge = 300,
    Error = 400,
    Failure = 500,
}

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub enum SmtpReplyCategory {
    Syntax = 0,
    Information = 10,
    Connections = 20,
    Reserved3 = 30,
    Reserved4 = 40,
    System = 50,
}

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub enum SmtpReplyDigit {
    D0 = 0,
    D1 = 1,
    D2 = 2,
    D3 = 3,
    D4 = 4,
    D5 = 5,
    D6 = 6,
    D7 = 7,
    D8 = 8,
    D9 = 9,
}
