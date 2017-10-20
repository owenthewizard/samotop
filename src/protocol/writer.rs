use std::io;
use std::io::Write;
use model::response::*;
use model::response::SmtpReply::*;

static SERIALIZER: SmtpSerializer = SmtpSerializer;

type Result = io::Result<()>;

pub trait SmtpAnswerSerializer {
    fn write(&self, buf: &mut Write, answer: SmtpReply) -> Result;
}

pub struct SmtpSerializer;

impl SmtpSerializer {
    pub fn answer_serializer<'a>() -> &'a SmtpAnswerSerializer {
        &SERIALIZER
    }

    fn reply_code(&self, reply: &SmtpReply) -> u16 {
        match reply {
            &Custom(ref class, ref category, ref digit, _, _) => {
                *class as u16 + *category as u16 + *digit as u16
            }

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
            &OkInfo(_, _) => 250,
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

    fn reply_text(&self, reply: &SmtpReply) -> String {
        match reply {
            &Custom(_, _, _, ref text, _) => format!("{}", text),

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

            &OkInfo(ref text, _) => {
                match text.is_empty() {
                    true => "Ok".to_owned(),
                    _ => format!("{}", text),
                }
            }
            &UserNotLocalInfo(ref forward_path) => {
                format!("User not local, will forward to {}", forward_path)
            }
            &CannotVerifyUserInfo => {
                "Cannot VFRY user, but will accept message and attempt delivery".to_owned()
            }
            &StartMailInputChallenge => "Start mail input, // end with <CRLF>.<CRLF>".to_owned(),
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

    fn reply_items(&self, reply: SmtpReply) -> Vec<String> {
        match reply {
            Custom(_, _, _, _, items) => items,
            OkInfo(_, items) => items,
            _ => vec![],
        }
    }

    fn write_reply(&self, mut buf: &mut Write, reply: SmtpReply) -> Result {
        let code = self.reply_code(&reply);
        let text = self.reply_text(&reply);
        let items = self.reply_items(reply);
        if items.is_empty() {
            try!(self.write_reply_end(&mut buf, code, &text));
        } else {
            try!(self.write_reply_continued(&mut buf, code, &text));
            for i in 0..items.len() {
                if i == items.len() - 1 {
                    try!(self.write_reply_end(&mut buf, code, &items[i]));
                } else {
                    try!(self.write_reply_continued(&mut buf, code, &items[i]));
                }
            }
        }
        buf.write_all(b"\r\n")
    }

    fn write_reply_end(&self, buf: &mut Write, code: u16, text: &str) -> Result {
        write!(buf, "{} {}", code, text)
    }
    fn write_reply_continued(&self, buf: &mut Write, code: u16, text: &str) -> Result {
        write!(buf, "{}-{}", code, text)
    }
}

impl SmtpAnswerSerializer for SmtpSerializer {
    fn write(&self, mut buf: &mut Write, reply: SmtpReply) -> Result {
        self.write_reply(&mut buf, reply)
    }
}
