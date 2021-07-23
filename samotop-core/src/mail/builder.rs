use crate::{
    common::*,
    io::tls::MayBeTls,
    mail::{
        AcceptsDispatch, AcceptsEsmtp, AcceptsGuard, AcceptsInterpret, AddRecipientRequest,
        AddRecipientResult, DispatchResult, MailDispatch, MailGuard, MailSetup, Service,
        StartMailRequest, StartMailResult,
    },
    smtp::{EsmtpService, Interpret, Interpretter, SessionInfo, SmtpState, Transaction},
};
use std::{
    ops::{Add, AddAssign},
    time::Duration,
};

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

impl AcceptsInterpret for Configuration {
    fn add_interpret<T: Interpret + Send + Sync + 'static>(&mut self, interpret: T) {
        self.interpret.insert(0, Box::new(interpret));
    }

    fn add_interpret_fallback<T: Interpret + Send + Sync + 'static>(&mut self, interpret: T) {
        self.interpret.push(Box::new(interpret))
    }

    fn wrap_interprets<
        T: Interpret + Send + Sync + 'static,
        F: Fn(Box<dyn Interpret + Send + Sync>) -> T,
    >(
        &mut self,
        wrap: F,
    ) {
        let interpret = wrap(Box::new(Interpretter::new(std::mem::take(
            &mut self.interpret,
        ))));
        self.interpret.push(Box::new(interpret))
    }
}

impl AcceptsEsmtp for Configuration {
    fn add_esmtp<T: EsmtpService + Send + Sync + 'static>(&mut self, esmtp: T) {
        self.esmtp.insert(0, Box::new(esmtp));
    }

    fn add_esmtp_fallback<T: EsmtpService + Send + Sync + 'static>(&mut self, esmtp: T) {
        self.esmtp.push(Box::new(esmtp))
    }

    fn wrap_esmtps<
        T: EsmtpService + Send + Sync + 'static,
        F: Fn(Box<dyn EsmtpService + Send + Sync>) -> T,
    >(
        &mut self,
        wrap: F,
    ) {
        let items = std::mem::take(&mut self.esmtp);
        let esmtp = wrap(Box::new(EsmtpBunch {
            id: time_based_id(),
            items,
        }));
        self.esmtp.push(Box::new(esmtp))
    }
}

impl AcceptsGuard for Configuration {
    fn add_guard<T: MailGuard + Send + Sync + 'static>(&mut self, guard: T) {
        self.guard.insert(0, Box::new(guard));
    }

    fn add_guard_fallback<T: MailGuard + Send + Sync + 'static>(&mut self, guard: T) {
        self.guard.push(Box::new(guard))
    }

    fn wrap_guards<
        T: MailGuard + Send + Sync + 'static,
        F: Fn(Box<dyn MailGuard + Send + Sync>) -> T,
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
    fn add_dispatch<T: MailDispatch + Send + Sync + 'static>(&mut self, dispatch: T) {
        self.dispatch.insert(0, Box::new(dispatch));
    }

    fn add_dispatch_fallback<T: MailDispatch + Send + Sync + 'static>(&mut self, dispatch: T) {
        self.dispatch.push(Box::new(dispatch))
    }

    fn wrap_dispatches<
        T: MailDispatch + Send + Sync + 'static,
        F: Fn(Box<dyn MailDispatch + Send + Sync>) -> T,
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
    /// Finalize and produce the MailService.
    pub fn build(self) -> Service {
        let Configuration {
            id,
            esmtp,
            interpret,
            guard,
            dispatch,
        } = self.config;
        Service::new(
            EsmtpBunch {
                id: id.clone(),
                items: esmtp,
            },
            Interpretter::new(interpret),
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
    interpret: Vec<Box<dyn Interpret + Send + Sync>>,
    dispatch: Vec<Box<dyn MailDispatch + Sync + Send + 'static>>,
    guard: Vec<Box<dyn MailGuard + Sync + Send + 'static>>,
    esmtp: Vec<Box<dyn EsmtpService + Sync + Send + 'static>>,
}
impl Default for Configuration {
    fn default() -> Self {
        Self {
            id: time_based_id(),
            interpret: Default::default(),
            dispatch: Default::default(),
            guard: Default::default(),
            esmtp: Default::default(),
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
    items: Vec<Box<dyn EsmtpService + Sync + Send>>,
}

impl EsmtpService for EsmtpBunch {
    fn read_timeout(&self) -> Option<std::time::Duration> {
        self.items.iter().fold(None, |timeout, svc| {
            svc.read_timeout()
                .map(|dur| {
                    timeout
                        .map(|timeout| Duration::min(dur, timeout))
                        .unwrap_or(dur)
                })
                .or(timeout)
        })
    }
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
                self.id,
                self.items.len(),
                state.session
            );
            for esmtp in self.items.iter() {
                trace!("Esmtp {} prepare_session calling {:?}", self.id, esmtp);
                esmtp.prepare_session(io, state).await;
            }

            if state.session.service_name.is_empty() {
                state.session.service_name = format!("Samotop-{}", self.id);
                warn!(
                    "Esmtp {} service name is empty. Using default {:?}",
                    self.id, state.session.service_name
                );
            } else {
                info!("Service name is {:?}", state.session.service_name);
            }
        })
    }
}
