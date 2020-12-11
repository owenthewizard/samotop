use crate::mail::{SessionInfo, Transaction};
use crate::{common::*, smtp::SmtpPath};

use super::Recipient;

/**
A mail guard can be queried whether a recepient is accepted on which address.
*/
pub trait MailGuard: fmt::Debug {
    fn add_recipient<'a, 'f>(
        &'a self,
        request: AddRecipientRequest,
    ) -> S2Fut<'f, AddRecipientResult>
    where
        'a: 'f;
    fn start_mail<'a, 's, 'f>(
        &'a self,
        session: &'s SessionInfo,
        request: StartMailRequest,
    ) -> S2Fut<'f, StartMailResult>
    where
        'a: 'f,
        's: 'f;
}

impl<T> MailGuard for Arc<T>
where
    T: MailGuard,
{
    fn add_recipient<'a, 'f>(
        &'a self,
        request: AddRecipientRequest,
    ) -> S2Fut<'f, AddRecipientResult>
    where
        'a: 'f,
    {
        T::add_recipient(self, request)
    }
    fn start_mail<'a, 's, 'f>(
        &'a self,
        session: &'s SessionInfo,
        request: StartMailRequest,
    ) -> S2Fut<'f, StartMailResult>
    where
        'a: 'f,
        's: 'f,
    {
        T::start_mail(self, session, request)
    }
}

pub type StartMailRequest = Transaction;

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum StartMailResult {
    /// Failure with explanation that should include the ID
    Failed(StartMailFailure, String),
    /// 250 Mail command accepted
    Accepted(Transaction),
}

#[derive(Debug, Clone)]
pub enum StartMailFailure {
    /// The whole mail transaction failed, subsequent RCPT and DATA will fail
    /// 421  <domain> Service not available, closing transmission channel
    ///  (This may be a reply to any command if the service knows it must
    ///    shut down)
    TerminateSession,
    /// 550 Requested action not taken: mailbox unavailable (e.g., mailbox
    /// not found, no access, or command rejected for policy reasons)
    Rejected,
    /// 553  Requested action not taken: mailbox name not allowed (e.g.,
    /// mailbox syntax incorrect)
    InvalidSender,
    /// 552  Requested mail action aborted: exceeded storage allocation
    StorageExhaustedPermanently,
    /// 452  Requested action not taken: insufficient system storage
    StorageExhaustedTemporarily,
    /// 451  Requested action aborted: local error in processing
    FailedTemporarily,
    /// 555  MAIL FROM/RCPT TO parameters not recognized or not implemented
    InvalidParameter,
    /// 455  Server unable to accommodate parameters
    InvalidParameterValue,
}

/// Request to check if mail is accepted for given recipient
#[derive(Debug)]
pub struct AddRecipientRequest {
    /// The envelope to add to
    pub transaction: Transaction,
    /// The SMTP rcpt to:path sent by peer we want to check
    pub rcpt: Recipient,
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum AddRecipientResult {
    Inconclusive(AddRecipientRequest),
    /// The whole mail transaction failed, subsequent RCPT and DATA will fail
    /// 421  <domain> Service not available, closing transmission channel
    ///  (This may be a reply to any command if the service knows it must
    ///    shut down)
    TerminateSession(String),
    /// Failed with description that should include the ID, see `AddRecipientFailure`
    Failed(Transaction, AddRecipientFailure, String),
    /// 251  User not local; will forward to <forward-path>
    AcceptedWithNewPath(Transaction, SmtpPath),
    /// 250  Requested mail action okay, completed
    Accepted(Transaction),
}

#[derive(Debug, Clone)]
pub enum AddRecipientFailure {
    /// 550 Requested action not taken: mailbox unavailable (e.g., mailbox
    /// not found, no access, or command rejected for policy reasons)
    RejectedPermanently,
    /// 450  Requested mail action not taken: mailbox unavailable (e.g.,
    /// mailbox busy or temporarily blocked for policy reasons)
    RejectedTemporarily,
    /// 551  User not local; please try <forward-path> (See Section 3.4)
    Moved(SmtpPath),
    /// 553  Requested action not taken: mailbox name not allowed (e.g.,
    /// mailbox syntax incorrect)
    InvalidRecipient,
    /// 552  Requested mail action aborted: exceeded storage allocation
    StorageExhaustedPermanently,
    /// 452  Requested action not taken: insufficient system storage
    StorageExhaustedTemporarily,
    /// 451  Requested action aborted: local error in processing
    FailedTemporarily,
    /// 555  MAIL FROM/RCPT TO parameters not recognized or not implemented
    InvalidParameter,
    /// 455  Server unable to accommodate parameters
    InvalidParameterValue,
}
