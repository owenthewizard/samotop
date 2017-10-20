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

#[derive(Eq, PartialEq, Debug)]
pub enum SmtpReply {
    // I'm using a suffix to make names sound english:
    // 2xx => ...Info
    // 3xx => ...Challenge
    // 4xx => ...Error
    // 5xx => ...Failure

    // other custom replies
    Custom(SmtpReplyClass, SmtpReplyCategory, SmtpReplyDigit, String, Vec<String>),

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
    OkInfo(String, Vec<String>),
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
