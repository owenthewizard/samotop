use crate::{
    common::*,
    io::tls::{NoTls, TlsProvider, TlsUpgrade},
    mail::{
        AddRecipientRequest, AddRecipientResult, DispatchResult, EsmtpService, MailDispatch,
        MailGuard, MailService, MailSetup, SessionInfo, StartMailRequest, StartMailResult,
        Transaction,
    },
    parser::{Dummy, Interpret, Parser},
};

use super::ParserProvider;

#[derive(Default)]
pub struct Builder {
    config: Configuration,
}

#[derive(Debug)]
pub struct Configuration {
    pub id: String,
    pub tls: Box<dyn TlsProvider + Sync + Send + 'static>,
    pub interpretter: Box<Arc<dyn Interpret + Send + Sync>>,
    pub data_parser: Vec<Arc<dyn Parser + Sync + Send + 'static>>,
    pub command_parser: Vec<Arc<dyn Parser + Sync + Send + 'static>>,
    pub dispatch: Vec<Box<dyn MailDispatch + Sync + Send + 'static>>,
    pub guard: Vec<Box<dyn MailGuard + Sync + Send + 'static>>,
    pub esmtp: Vec<Box<dyn EsmtpService + Sync + Send + 'static>>,
}

impl Builder {
    pub fn using(mut self, setup: impl MailSetup) -> Self {
        trace!("Builder {} using setup {:?}", self.config.id, setup);
        setup.setup(&mut self.config);
        self
    }
    pub fn into_service(self) -> impl MailService {
        self.config
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            id: Default::default(),
            tls: Box::new(NoTls),
            interpretter: Box::new(Arc::new(Dummy)),
            data_parser: Default::default(),
            command_parser: Default::default(),
            dispatch: Default::default(),
            guard: Default::default(),
            esmtp: Default::default(),
        }
    }
}

impl MailDispatch for Configuration {
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
            self.dispatch.len(),
            transaction,
            session
        );
        let fut = async move {
            for disp in self.dispatch.iter() {
                trace!("Dispatch {} send_mail calling {:?}", self.id, disp);
                transaction = disp.send_mail(session, transaction).await?;
            }
            Ok(transaction)
        };
        Box::pin(fut)
    }
}

impl MailGuard for Configuration {
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
            self.guard.len(),
            request
        );
        let fut = async move {
            for guard in self.guard.iter() {
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
            self.guard.len(),
            request
        );
        let fut = async move {
            for guard in self.guard.iter() {
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

impl EsmtpService for Configuration {
    fn prepare_session(&self, session: &mut SessionInfo) {
        debug!(
            "Esmtp {} with {} esmtps preparing session {:?}",
            self.id,
            self.esmtp.len(),
            session
        );
        for esmtp in self.esmtp.iter() {
            trace!("Esmtp {} prepare_session calling {:?}", self.id, esmtp);
            esmtp.prepare_session(session);
        }
    }
}

impl TlsProvider for Configuration {
    fn get_tls_upgrade(&self) -> Option<Box<dyn TlsUpgrade>> {
        self.tls.get_tls_upgrade()
    }
}

impl ParserProvider for Configuration {
    fn get_parser_for_data(&self) -> Box<dyn Parser + Sync + Send> {
        Box::new(self.data_parser.clone())
    }

    fn get_parser_for_commands(&self) -> Box<dyn Parser + Sync + Send> {
        Box::new(self.command_parser.clone())
    }

    fn get_interpretter(&self) -> Box<dyn Interpret + Sync + Send> {
        Box::new(self.interpretter.clone())
    }
}
