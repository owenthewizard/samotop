use crate::{
    common::*,
    io::{tls::MayBeTls, ConnectionInfo, Handler},
    mail::{
        AddRecipientResult, DispatchResult, MailDispatch, MailGuard, Recipient, StartMailResult,
    },
    smtp::{Drive, SessionSetup, SmtpContext, SmtpSession, SessionSetupService},
    store::{Component, SingleComponent, Store},
};

/// A short hand for all the mandatory mail services
pub trait MailService: SessionSetup + MailGuard + MailDispatch {}
impl<T> MailService for T where T: SessionSetup + MailGuard + MailDispatch {}

pub struct MailSvc {}
impl Component for MailSvc {
    type Target = Box<dyn MailService + Send + Sync + 'static>;
}
impl SingleComponent for MailSvc {}

/// Service implements all the mandatory mail services
/// + IoService so it can be used with `TcpServer` or `UnixServer`.
///
/// Build it using the `Builder`
#[derive(Debug)]
pub struct Service {
    store: Store,
    session: Arc<dyn SessionSetup + Sync + Send>,
    guard: Arc<dyn MailGuard + Sync + Send>,
    dispatch: Arc<dyn MailDispatch + Sync + Send>,
    driver: Arc<dyn Drive + Sync + Send>,
}

impl Service {
    /// Compose the service from parts
    pub fn new<T, E, G, D>(store: Store, drive: T, session: E, guard: G, dispatch: D) -> Self
    where
        T: Drive + Sync + Send + 'static,
        E: SessionSetup + Sync + Send + 'static,
        G: MailGuard + Sync + Send + 'static,
        D: MailDispatch + Sync + Send + 'static,
    {
        Self {
            store,
            session: Arc::new(session),
            dispatch: Arc::new(dispatch),
            guard: Arc::new(guard),
            driver: Arc::new(drive),
        }
    }
}
impl Handler for Service {
    fn handle(
        &self,
        io: Result<Box<dyn MayBeTls>>,
        connection: ConnectionInfo,
    ) -> S1Fut<'static, Result<()>> {
        if let Some(setup) = self.store.get_or_compose::<SessionSetupService>() {
            Box::pin(async move {
                let store = Store::default();
                setup.setup_session().await;
                let driver = self.driver.clone();
                // fetch and apply commands
                driver.drive(&mut io?, connection, store).await?;
                Ok(())
            })
        }

        trace!("New peer connection {}", connection);

        Box::pin(async move {
            // fetch and apply commands
            driver.drive(&mut io?, connection, store).await?;
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

impl SessionSetup for Service {
    fn setup_session<'a, 'i, 's, 'f>(
        &'a self,
        io: &'i mut Box<dyn MayBeTls>,
        state: &'s mut SmtpContext,
    ) -> S1Fut<'f, ()>
    where
        'a: 'f,
        'i: 'f,
        's: 'f,
    {
        self.session.setup_session(io, state)
    }
}
