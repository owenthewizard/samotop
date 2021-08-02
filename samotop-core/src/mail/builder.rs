use crate::{
    common::*,
    io::tls::MayBeTls,
    mail::{
        AcceptsDispatch, AcceptsGuard, AcceptsInterpretter, AcceptsSessionService,
        AddRecipientRequest, AddRecipientResult, DispatchResult, MailDispatch, MailGuard,
        MailSetup, Service, StartMailRequest, StartMailResult,
    },
    smtp::{Drive, Interpret, Interpretter, SessionInfo, SessionService, SmtpState, Transaction},
};
use std::ops::{Add, AddAssign};

/// Builds MailService from components
#[derive(Default)]
pub struct Builder;

#[derive(Default)]
pub struct BuilderWithConfig {
    config: Configuration,
}

impl<T: MailSetup<Configuration>> Add<T> for Builder {
    type Output = BuilderWithConfig;

    fn add(self, setup: T) -> Self::Output {
        BuilderWithConfig::default() + setup
    }
}
impl<T: MailSetup<Configuration>> Add<T> for BuilderWithConfig {
    type Output = Self;

    fn add(mut self, setup: T) -> Self::Output {
        self += setup;
        self
    }
}
impl<T: MailSetup<Configuration>> AddAssign<T> for BuilderWithConfig {
    fn add_assign(&mut self, setup: T) {
        trace!("Service builder {} using setup {:?}", self.config.id, setup);
        setup.setup(&mut self.config)
    }
}

impl AcceptsSessionService for Configuration {
    fn add_first_session_service<T: SessionService + Send + Sync + 'static>(&mut self, session: T) {
        self.session.insert(0, Box::new(session));
    }

    fn add_last_session_service<T: SessionService + Send + Sync + 'static>(&mut self, session: T) {
        self.session.push(Box::new(session))
    }

    fn wrap_session_service<
        T: SessionService + Send + Sync + 'static,
        F: FnOnce(Box<dyn SessionService + Send + Sync>) -> T,
    >(
        &mut self,
        wrap: F,
    ) {
        let items = std::mem::take(&mut self.session);
        let session = wrap(Box::new(EsmtpBunch {
            id: time_based_id(),
            items,
        }));
        self.session.push(Box::new(session))
    }
}

impl AcceptsGuard for Configuration {
    fn add_first_guard<T: MailGuard + Send + Sync + 'static>(&mut self, guard: T) {
        self.guard.insert(0, Box::new(guard));
    }

    fn add_last_guard<T: MailGuard + Send + Sync + 'static>(&mut self, guard: T) {
        self.guard.push(Box::new(guard))
    }

    fn wrap_guards<
        T: MailGuard + Send + Sync + 'static,
        F: FnOnce(Box<dyn MailGuard + Send + Sync>) -> T,
    >(
        &mut self,
        wrap: F,
    ) {
        let items = std::mem::take(&mut self.guard);
        let guard = wrap(Box::new(GuardBunch {
            id: time_based_id(),
            items,
        }));
        self.guard.push(Box::new(guard))
    }
}

impl AcceptsDispatch for Configuration {
    fn add_first_dispatch<T: MailDispatch + Send + Sync + 'static>(&mut self, dispatch: T) {
        self.dispatch.insert(0, Box::new(dispatch));
    }

    fn add_last_dispatch<T: MailDispatch + Send + Sync + 'static>(&mut self, dispatch: T) {
        self.dispatch.push(Box::new(dispatch))
    }

    fn wrap_dispatches<
        T: MailDispatch + Send + Sync + 'static,
        F: FnOnce(Box<dyn MailDispatch + Send + Sync>) -> T,
    >(
        &mut self,
        wrap: F,
    ) {
        let items = std::mem::take(&mut self.dispatch);
        let dispatch = wrap(Box::new(DispatchBunch {
            id: time_based_id(),
            items,
        }));
        self.dispatch.push(Box::new(dispatch))
    }
}

impl AcceptsInterpretter for Configuration {
    fn add_first_interpretter<T: Interpret + Send + Sync + 'static>(&mut self, item: T) {
        self.interpret.insert(0, Box::new(item));
    }

    fn add_last_interpretter<T: Interpret + Send + Sync + 'static>(&mut self, item: T) {
        self.interpret.push(Box::new(item))
    }

    fn wrap_interpretter<
        T: Interpret + Send + Sync + 'static,
        F: FnOnce(Box<dyn Interpret + Send + Sync>) -> T,
    >(
        &mut self,
        wrap: F,
    ) {
        let items = std::mem::take(&mut self.interpret);
        self.interpret
            .push(Box::new(wrap(Box::new(Interpretter::new(items)))));
    }
}
impl Builder {
    pub fn empty() -> BuilderWithConfig {
        BuilderWithConfig::default()
    }
    /// Use a given MailSetup to build a MailService.
    ///
    /// See MailSetup for examples.
    pub fn using(self, setup: impl MailSetup<Configuration>) -> BuilderWithConfig {
        BuilderWithConfig::default() + setup
    }
    /// Finalize and produce the MailService.
    pub fn build(self) -> Service {
        BuilderWithConfig::default().build()
    }
}
impl BuilderWithConfig {
    /// Use a given MailSetup to build a MailService.
    ///
    /// See MailSetup for examples.
    pub fn using(self, setup: impl MailSetup<Configuration>) -> Self {
        self + setup
    }
    #[cfg(feature = "driver")]
    /// Finalize and produce the MailService.
    pub fn build(self) -> Service {
        self.build_with_driver(crate::smtp::SmtpDriver)
    }

    /// Finalize and produce the MailService.
    pub fn build_with_driver(self, driver: impl Drive + Send + Sync + 'static) -> Service {
        let Configuration {
            id,
            session,
            guard,
            dispatch,
            interpret,
        } = self.config;
        Service::new(
            driver,
            Interpretter::new(interpret),
            EsmtpBunch {
                id: id.clone(),
                items: session,
            },
            GuardBunch {
                id: id.clone(),
                items: guard,
            },
            DispatchBunch {
                id,
                items: dispatch,
            },
        )
    }
}

/// Service builder configuration
#[derive(Debug)]
pub struct Configuration {
    /// ID used for identifying this instance in logs
    pub id: String,
    dispatch: Vec<Box<dyn MailDispatch + Sync + Send + 'static>>,
    guard: Vec<Box<dyn MailGuard + Sync + Send + 'static>>,
    session: Vec<Box<dyn SessionService + Sync + Send + 'static>>,
    interpret: Vec<Box<dyn Interpret + Sync + Send + 'static>>,
}
impl Default for Configuration {
    fn default() -> Self {
        Self {
            id: time_based_id(),
            dispatch: Default::default(),
            guard: Default::default(),
            session: Default::default(),
            interpret: Default::default(),
        }
    }
}

#[derive(Debug)]
struct DispatchBunch {
    id: String,
    items: Vec<Box<dyn MailDispatch + Sync + Send>>,
}

impl MailDispatch for DispatchBunch {
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
            self.id,
            self.items.len(),
            transaction,
            session
        );
        let fut = async move {
            for disp in self.items.iter() {
                trace!("Dispatch {} send_mail calling {:?}", self.id, disp);
                transaction = disp.send_mail(session, transaction).await?;
            }
            Ok(transaction)
        };
        Box::pin(fut)
    }
}

#[derive(Debug)]
struct GuardBunch {
    id: String,
    items: Vec<Box<dyn MailGuard + Sync + Send>>,
}

impl MailGuard for GuardBunch {
    fn add_recipient<'a, 'f>(
        &'a self,
        mut request: AddRecipientRequest,
    ) -> S2Fut<'f, AddRecipientResult>
    where
        'a: 'f,
    {
        debug!(
            "Guard {} with {} guards adding recipient {:?}",
            self.id,
            self.items.len(),
            request
        );
        let fut = async move {
            for guard in self.items.iter() {
                trace!("Guard {} add_recipient calling {:?}", self.id, guard);
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
            self.id,
            self.items.len(),
            request
        );
        let fut = async move {
            for guard in self.items.iter() {
                trace!("Guard {} start_mail calling {:?}", self.id, guard);
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

#[derive(Debug)]
struct EsmtpBunch {
    id: String,
    items: Vec<Box<dyn SessionService + Sync + Send>>,
}

impl SessionService for EsmtpBunch {
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
                "SessionService {} with {} children preparing session {:?}",
                self.id,
                self.items.len(),
                state.session
            );

            for svc in self.items.iter() {
                trace!(
                    "SessionService {} prepare_session calling {:?}",
                    self.id,
                    svc
                );
                svc.prepare_session(io, state).await;
            }

            if state.session.service_name.is_empty() {
                state.session.service_name = format!("Samotop-{}", self.id);
                warn!(
                    "SessionService {} service name is empty. Using default {:?}",
                    self.id, state.session.service_name
                );
            } else {
                info!(
                    "SessionService service name is {:?}",
                    state.session.service_name
                );
            }
        })
    }
}
