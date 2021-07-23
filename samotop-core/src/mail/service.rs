use std::time::Duration;

use crate::{
    common::*,
    io::tls::MayBeTls,
    mail::{
        AddRecipientRequest, AddRecipientResult, DispatchResult, DriverProvider, MailDispatch,
        MailGuard, StartMailRequest, StartMailResult,
    },
    smtp::{EsmtpService, Interpret, SessionInfo, SmtpDriver, SmtpState, Transaction},
};

#[derive(Debug, Clone)]
pub struct Service {
    esmtp: Arc<dyn EsmtpService + Sync + Send>,
    interpret: Arc<dyn Interpret + Sync + Send>,
    guard: Arc<dyn MailGuard + Sync + Send>,
    dispatch: Arc<dyn MailDispatch + Sync + Send>,
}

impl Service {
    pub fn new<E, I, G, D>(esmtp: E, interpret: I, guard: G, dispatch: D) -> Self
    where
        E: EsmtpService + Sync + Send + 'static,
        I: Interpret + Sync + Send + 'static,
        G: MailGuard + Sync + Send + 'static,
        D: MailDispatch + Sync + Send + 'static,
    {
        Self {
            esmtp: Arc::new(esmtp),
            interpret: Arc::new(interpret),
            dispatch: Arc::new(dispatch),
            guard: Arc::new(guard),
        }
    }
}

impl MailDispatch for Service {
    fn send_mail<'a, 's, 'f>(
        &'a self,
        session: &'s SessionInfo,
        transaction: Transaction,
    ) -> S1Fut<'f, DispatchResult>
    where
        'a: 'f,
        's: 'f,
    {
        self.dispatch.send_mail(session, transaction)
    }
}

impl MailGuard for Service {
    fn add_recipient<'a, 'f>(
        &'a self,
        request: AddRecipientRequest,
    ) -> S2Fut<'f, AddRecipientResult>
    where
        'a: 'f,
    {
        self.guard.add_recipient(request)
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
        self.guard.start_mail(session, request)
    }
}

impl EsmtpService for Service {
    fn read_timeout(&self) -> Option<Duration> {
        self.esmtp.read_timeout()
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
        self.esmtp.prepare_session(io, state)
    }
}

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
        self.interpret.interpret(input, state)
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
