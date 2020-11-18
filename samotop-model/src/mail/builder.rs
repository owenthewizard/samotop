use crate::{
    common::S2Fut,
    mail::{
        AddRecipientRequest, AddRecipientResult, DispatchError, DispatchResult, EsmtpService,
        MailDispatch, MailGuard, MailSetup, SessionInfo, StartMailRequest, StartMailResult,
        Transaction,
    },
};

#[derive(Default)]
pub struct Builder {
    pub dispatch: Vec<Box<dyn MailDispatch + Sync + Send + 'static>>,
    pub guard: Vec<Box<dyn MailGuard + Sync + Send + 'static>>,
    pub esmtp: Vec<Box<dyn EsmtpService + Sync + Send + 'static>>,
}

impl Builder {
    pub fn using(mut self, setup: impl MailSetup) -> Self {
        setup.setup(&mut self);
        self
    }
}

impl MailDispatch for Builder {
    fn send_mail<'a, 's, 'f>(
        &'a self,
        session: &'s SessionInfo,
        mut transaction: Transaction,
    ) -> S2Fut<'f, DispatchResult>
    where
        'a: 'f,
        's: 'f,
    {
        let fut = async move {
            for disp in self.dispatch.iter() {
                match disp.send_mail(session, transaction).await {
                    Ok(t) => {
                        transaction = t;
                        if transaction.sink.is_some() {
                            return Ok(transaction);
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
            Err(DispatchError::Refused)
        };
        Box::pin(fut)
    }
}

impl MailGuard for Builder {
    fn add_recipient<'a, 'f>(
        &'a self,
        mut request: AddRecipientRequest,
    ) -> S2Fut<'f, AddRecipientResult>
    where
        'a: 'f,
    {
        let fut = async move {
            for guard in self.guard.iter() {
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
        let fut = async move {
            for guard in self.guard.iter() {
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

impl EsmtpService for Builder {
    fn prepare_session(&self, session: &mut SessionInfo) {
        for esmtp in self.esmtp.iter() {
            esmtp.prepare_session(session);
        }
    }
}
