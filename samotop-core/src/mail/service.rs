use crate::{
    common::*,
    io::{tls::MayBeTls, ConnectionInfo, IoService},
    mail::{
        AddRecipientRequest, AddRecipientResult, Configuration, DispatchResult, DriverProvider,
        MailDispatch, MailGuard, StartMailRequest, StartMailResult,
    },
    smtp::{
        interpret_all, EsmtpService, Interpret, SessionInfo, SmtpDriver, SmtpState, Transaction,
    },
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
            self.config.logging_id,
            self.config.dispatch.len(),
            transaction,
            session
        );
        let fut = async move {
            for disp in self.config.dispatch.iter() {
                trace!(
                    "Dispatch {} send_mail calling {:?}",
                    self.config.logging_id,
                    disp
                );
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
            self.config.logging_id,
            self.config.guard.len(),
            request
        );
        let fut = async move {
            for guard in self.config.guard.iter() {
                trace!(
                    "Guard {} add_recipient calling {:?}",
                    self.config.logging_id,
                    guard
                );
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
            self.config.logging_id,
            self.config.guard.len(),
            request
        );
        let fut = async move {
            for guard in self.config.guard.iter() {
                trace!(
                    "Guard {} start_mail calling {:?}",
                    self.config.logging_id,
                    guard
                );
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
    fn prepare_session<'a, 'i, 's, 'f>(
        &'a self,
        io: &'i mut Box<dyn MayBeTls>,
        state: &'s mut SmtpState,
    ) -> S1Fut<'f, ()>
    where
        'a: 'f,
        'i: 'f,
        's: 'f,
    {
        Box::pin(async move {
            debug!(
                "Esmtp {} with {} esmtps preparing session {:?}",
                self.config.logging_id,
                self.config.esmtp.len(),
                state.session
            );
            for esmtp in self.config.esmtp.iter() {
                trace!(
                    "Esmtp {} prepare_session calling {:?}",
                    self.config.logging_id,
                    esmtp
                );
                esmtp.prepare_session(io, state).await;
            }

            if state.session.service_name.is_empty() {
                state.session.service_name = format!("Samotop-{}", self.config.logging_id);
                warn!(
                    "Esmtp {} service name is empty. Using default {:?}",
                    self.config.logging_id, state.session.service_name
                );
            } else {
                info!("Service name is {:?}", state.session.service_name);
            }
        })
    }
}

// Removed, use .using(EsmtpStartTls::...())
// impl TlsProvider for Service {
//     fn get_tls_upgrade(&self) -> Option<Box<dyn TlsUpgrade>> {
//         self.config.tls.get_tls_upgrade()
//     }
// }

impl Interpret for Service {
    fn interpret<'a, 'i, 's, 'f>(
        &'a self,
        input: &'i [u8],
        state: &'s mut SmtpState,
    ) -> S1Fut<'f, crate::smtp::InterpretResult>
    where
        'a: 'f,
        'i: 'f,
        's: 'f,
    {
        Box::pin(interpret_all(
            self.config.interpret.as_slice(),
            input,
            state,
        ))
    }
}

impl DriverProvider for Service {
    fn get_driver<'io>(
        &self,
        io: &'io mut dyn MayBeTls,
    ) -> Box<dyn crate::smtp::Drive + Sync + Send + 'io> {
        Box::new(SmtpDriver::new(io))
    }

    fn get_interpretter(&self) -> Box<dyn Interpret + Sync + Send> {
        Box::new(self.clone())
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
            let mut state = SmtpState::new(service.clone());
            state.session.connection = connection;

            service.prepare_session(&mut io, &mut state).await;

            // fetch and apply commands
            state.service.get_driver(&mut io).drive(&mut state).await?;

            Ok(())
        })
    }
}
