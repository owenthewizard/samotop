use crate::{
    common::*,
    mail::Recipient,
    smtp::{SmtpPath, SmtpSession},
    config::{Component, ComposableComponent, MultiComponent},
};

/**
A mail guard opens the mail transaction after a MAIL command - `start_mail`.
It will then be queried whether each individual recepient (RCPT command) is accepted on which address.
It can also modify the recipient address with an optional notification back to the client.
*/
pub trait MailGuard: fmt::Debug {
    /// Open the mail transaction. Here we have the opportunity to check the sender, adjust transaction ID...
    fn start_mail<'a, 's, 'f>(&'a self, session: &'s mut SmtpSession) -> S2Fut<'f, StartMailResult>
    where
        'a: 'f,
        's: 'f;
    /// Add given RCPT to the list. Here we can immediately add and further processing will stop with success.
    /// Or we can refuse and stop further processing with a failure.
    /// Last option is to return `Inconclusive` in which case other MailGuards will have a chance.
    /// If all MailGuards return `Inconclusive`, the caller should assume success and add the RCPT to the list.
    fn add_recipient<'a, 's, 'f>(
        &'a self,
        session: &'s mut SmtpSession,
        rcpt: Recipient,
    ) -> S2Fut<'f, AddRecipientResult>
    where
        'a: 'f,
        's: 'f;
}
pub struct MailGuardService {}
impl Component for MailGuardService {
    type Target = Arc<dyn MailGuard + Send + Sync>;
}
impl MultiComponent for MailGuardService {}
impl ComposableComponent for MailGuardService {
    fn from_none() -> Self::Target {
        Self::from_many(vec![])
    }

    fn from_many(options: Vec<Self::Target>) -> Self::Target {
        Arc::new(options)
    }
}

impl MailGuard for Vec<<MailGuardService as Component>::Target> {
    fn add_recipient<'a, 's, 'f>(
        &'a self,
        session: &'s mut SmtpSession,
        mut rcpt: Recipient,
    ) -> S2Fut<'f, AddRecipientResult>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
            for guard in self.iter() {
                trace!("add_recipient calling {:?}", guard);
                match guard.add_recipient(session, rcpt).await {
                    AddRecipientResult::Inconclusive(r) => rcpt = r,
                    otherwise => return otherwise,
                }
            }
            FallBack.add_recipient(session, rcpt).await
        })
    }
    fn start_mail<'a, 's, 'f>(&'a self, session: &'s mut SmtpSession) -> S2Fut<'f, StartMailResult>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
            for guard in self.iter() {
                trace!("start_mail calling {:?}", guard);
                match guard.start_mail(session).await {
                    StartMailResult::Accepted => {}
                    otherwise => return otherwise,
                }
            }
            FallBack.start_mail(session).await
        })
    }
}

impl MailGuard for FallBack {
    /// Always inconclusive
    fn add_recipient<'a, 's, 'f>(
        &'a self,
        _session: &'s mut SmtpSession,
        rcpt: Recipient,
    ) -> S2Fut<'f, AddRecipientResult>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(ready(AddRecipientResult::Inconclusive(rcpt)))
    }
    /// Always accept
    fn start_mail<'a, 's, 'f>(&'a self, _session: &'s mut SmtpSession) -> S2Fut<'f, StartMailResult>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(ready(StartMailResult::Accepted))
    }
}

#[derive(Debug, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum StartMailResult {
    /// Failure with explanation that should include the ID
    Failed(StartMailFailure, String),
    /// 250 Mail command accepted
    Accepted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum AddRecipientResult {
    Inconclusive(Recipient),
    /// Failed with description that should include the ID, see `AddRecipientFailure`
    Failed(AddRecipientFailure, String),
    /// 251  User not local; will forward to <forward-path>
    AcceptedWithNewPath(SmtpPath),
    /// 250  Requested mail action okay, completed
    Accepted,
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
