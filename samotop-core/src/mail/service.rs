use crate::{
    common::*,
    io::{tls::MayBeTls, ConnectionInfo, IoService},
    mail::{
        AddRecipientResult, DispatchResult, MailDispatch, MailGuard, Recipient, StartMailResult,
    },
    smtp::{Drive, Interpret, SessionService, SmtpContext, SmtpSession},
};

/// A short hand for all the mandatory mail services
pub trait MailService: SessionService + MailGuard + MailDispatch {}
impl<T> MailService for T where T: SessionService + MailGuard + MailDispatch {}

/// Service implements all the mandatory mail services
/// + IoService so it can be used with `TcpServer` or `UnixServer`.
///
/// Build it using the `Builder`
#[derive(Debug, Clone)]
pub struct Service {
    session: Arc<dyn SessionService + Sync + Send>,
    guard: Arc<dyn MailGuard + Sync + Send>,
    dispatch: Arc<dyn MailDispatch + Sync + Send>,
    driver: Arc<dyn Drive + Sync + Send>,
    interpret: Arc<dyn Interpret + Sync + Send>,
}

impl Service {
    /// Compose the service from parts
    pub fn new<T, I, E, G, D>(drive: T, interpret: I, session: E, guard: G, dispatch: D) -> Self
    where
        T: Drive + Sync + Send + 'static,
        I: Interpret + Sync + Send + 'static,
        E: SessionService + Sync + Send + 'static,
        G: MailGuard + Sync + Send + 'static,
        D: MailDispatch + Sync + Send + 'static,
    {
        Self {
            session: Arc::new(session),
            dispatch: Arc::new(dispatch),
            guard: Arc::new(guard),
            driver: Arc::new(drive),
            interpret: Arc::new(interpret),
        }
    }
}

impl IoService for Service {
    fn handle(
        &self,
        io: Result<Box<dyn MayBeTls>>,
        connection: ConnectionInfo,
    ) -> S1Fut<'static, Result<()>> {
        let service = self.clone();
        let driver = self.driver.clone();
        let interpret = self.interpret.clone();

        trace!("New peer connection {}", connection);
        let mut state = SmtpContext::new(service, connection);

        Box::pin(async move {
            // fetch and apply commands
            driver.drive(&mut io?, &interpret, &mut state).await?;
            Ok(())
        })
    }
}

impl MailDispatch for Service {
    fn open_mail_body<'a, 's, 'f>(
        &'a self,
        session: &'s mut SmtpSession,
    ) -> S1Fut<'f, DispatchResult>
    where
        'a: 'f,
        's: 'f,
    {
        self.dispatch.open_mail_body(session)
    }
}

impl MailGuard for Service {
    fn add_recipient<'a, 's, 'f>(
        &'a self,
        session: &'s mut SmtpSession,
        rcpt: Recipient,
    ) -> S2Fut<'f, AddRecipientResult>
    where
        'a: 'f,
        's: 'f,
    {
        self.guard.add_recipient(session, rcpt)
    }

    fn start_mail<'a, 's, 'f>(&'a self, session: &'s mut SmtpSession) -> S2Fut<'f, StartMailResult>
    where
        'a: 'f,
        's: 'f,
    {
        self.guard.start_mail(session)
    }
}

impl SessionService for Service {
    fn prepare_session<'a, 'i, 's, 'f>(
        &'a self,
        io: &'i mut Box<dyn MayBeTls>,
        state: &'s mut SmtpContext,
    ) -> S1Fut<'f, ()>
    where
        'a: 'f,
        'i: 'f,
        's: 'f,
    {
        self.session.prepare_session(io, state)
    }
}
