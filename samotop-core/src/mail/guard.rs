use super::Recipient;
use crate::{
    common::*,
    smtp::{SessionInfo, SmtpPath, Transaction},
};
use std::ops::Deref;

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

impl<S: MailGuard + ?Sized, T: Deref<Target = S>> MailGuard for T
where
    T: fmt::Debug + Send + Sync,
    S: Sync,
{
    fn add_recipient<'a, 'f>(
        &'a self,
        request: AddRecipientRequest,
    ) -> S2Fut<'f, AddRecipientResult>
    where
        'a: 'f,
    {
        Box::pin(async move { S::add_recipient(Deref::deref(self), request).await })
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
        Box::pin(async move { S::start_mail(Deref::deref(self), session, request).await })
    }
}

impl MailGuard for Dummy {
    fn add_recipient<'a, 'f>(
        &'a self,
        request: AddRecipientRequest,
    ) -> S2Fut<'f, AddRecipientResult>
    where
        'a: 'f,
    {
        Box::pin(ready(AddRecipientResult::Inconclusive(request)))
    }

    fn start_mail<'a, 's, 'f>(
        &'a self,
        _session: &'s SessionInfo,
        _request: StartMailRequest,
    ) -> S2Fut<'f, StartMailResult>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(ready(StartMailResult::Failed(
            StartMailFailure::Rejected,
            "dummy".to_string(),
        )))
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
    /// Failed with description that should include the ID, see `AddRecipientFailure`
    Failed(Transaction, AddRecipientFailure, String),
    /// 251  User not local; will forward to <forward-path>
    AcceptedWithNewPath(Transaction, SmtpPath),
    /// 250  Requested mail action okay, completed
    Accepted(Transaction),
}

#[derive(Debug, Clone)]
pub enum AddRecipientFailure {
    /// The whole mail transaction failed, subsequent RCPT and DATA will fail
    /// 421  <domain> Service not available, closing transmission channel
    ///  (This may be a reply to any command if the service knows it must
    ///    shut down)
    TerminateSession,
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
