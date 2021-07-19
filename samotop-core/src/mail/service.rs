use crate::{
    common::*,
    io::{
        tls::{Io, MayBeTls, TlsCapable, TlsProvider, TlsUpgrade},
        ConnectionInfo, IoService,
    },
    mail::{
        AddRecipientRequest, AddRecipientResult, Configuration, DispatchResult, DriverIo,
        DriverProvider, EsmtpService, MailDispatch, MailGuard, SessionInfo, StartMailRequest,
        StartMailResult, Transaction,
    },
    smtp::{extension, Interpret, SmtpDriver, SmtpState},
};

#[derive(Default, Debug, Clone)]
pub struct Service {
    config: Arc<Configuration>,
}

impl Service {
    pub fn new(config: Configuration) -> Self {
        Self {
            config: Arc::new(config),
        }
    }
}

impl MailDispatch for Service {
    fn send_mail<'a, 's, 'f>(
        &'a self,
        session: &'s SessionInfo,
        mut transaction: Transaction,
    ) -> S1Fut<'f, DispatchResult>
    where
        'a: 'f,
        's: 'f,
    {
        debug!(
            "Dispatch {} with {} dispatchers sending mail {:?} on session {:?}",
            self.config.id,
            self.config.dispatch.len(),
            transaction,
            session
        );
        let fut = async move {
            for disp in self.config.dispatch.iter() {
                trace!("Dispatch {} send_mail calling {:?}", self.config.id, disp);
                transaction = disp.send_mail(session, transaction).await?;
            }
            Ok(transaction)
        };
        Box::pin(fut)
    }
}

impl MailGuard for Service {
    fn add_recipient<'a, 'f>(
        &'a self,
        mut request: AddRecipientRequest,
    ) -> S2Fut<'f, AddRecipientResult>
    where
        'a: 'f,
    {
        debug!(
            "Guard {} with {} guards adding recipient {:?}",
            self.config.id,
            self.config.guard.len(),
            request
        );
        let fut = async move {
            for guard in self.config.guard.iter() {
                trace!("Guard {} add_recipient calling {:?}", self.config.id, guard);
                match guard.add_recipient(request).await {
                    AddRecipientResult::Inconclusive(r) => request = r,
                    otherwise => return otherwise,
                }
            }
            AddRecipientResult::Inconclusive(request)
        };
        Box::pin(fut)
    }

    fn start_mail<'a, 's, 'f>(
        &'a self,
        session: &'s SessionInfo,
        mut request: StartMailRequest,
    ) -> S2Fut<'f, StartMailResult>
    where
        'a: 'f,
        's: 'f,
    {
        debug!(
            "Guard {} with {} guards starting mail {:?}",
            self.config.id,
            self.config.guard.len(),
            request
        );
        let fut = async move {
            for guard in self.config.guard.iter() {
                trace!("Guard {} start_mail calling {:?}", self.config.id, guard);
                match guard.start_mail(session, request).await {
                    StartMailResult::Accepted(r) => request = r,
                    otherwise => return otherwise,
                }
            }
            StartMailResult::Accepted(request)
        };
        Box::pin(fut)
    }
}

impl EsmtpService for Service {
    fn prepare_session(&self, session: &mut SessionInfo) {
        debug!(
            "Esmtp {} with {} esmtps preparing session {:?}",
            self.config.id,
            self.config.esmtp.len(),
            session
        );
        for esmtp in self.config.esmtp.iter() {
            trace!(
                "Esmtp {} prepare_session calling {:?}",
                self.config.id,
                esmtp
            );
            esmtp.prepare_session(session);
        }
    }
}

impl TlsProvider for Service {
    fn get_tls_upgrade(&self) -> Option<Box<dyn TlsUpgrade>> {
        self.config.tls.get_tls_upgrade()
    }
}

impl DriverProvider for Service {
    fn get_interpretter(&self) -> Box<dyn Interpret + Sync + Send> {
        Box::new(self.config.interpretter.clone())
    }

    fn get_driver<'io>(
        &self,
        io: &'io mut dyn DriverIo,
    ) -> Box<dyn crate::smtp::Drive + Sync + Send + 'io> {
        Box::new(SmtpDriver::new(io))
    }
}

impl IoService for Service {
    fn handle(
        &self,
        io: Result<Box<dyn MayBeTls>>,
        connection: ConnectionInfo,
    ) -> S1Fut<'static, Result<()>> {
        let service = self.clone();

        Box::pin(async move {
            info!("New peer connection {}", connection);
            let mut io = io?;
            let mut state = SmtpState::new(service);
            state.session.connection = connection;

            if !io.is_encrypted() {
                // Add tls if needed and available
                if !io.can_encrypt() {
                    if let Some(upgrade) = state.service.get_tls_upgrade() {
                        let plain: Box<dyn Io> = Box::new(io);
                        io = Box::new(TlsCapable::enabled(plain, upgrade, String::default()));
                    }
                }
                // enable STARTTLS extension if it can be used
                if io.can_encrypt() {
                    state.session.extensions.enable(&extension::STARTTLS);
                }
            }

            // fetch and apply commands
            state.service.get_driver(&mut io).drive(&mut state).await?;

            Ok(())
        })
    }
}
