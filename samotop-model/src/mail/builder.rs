use crate::{
    common::S2Fut,
    mail::{
        AddRecipientRequest, AddRecipientResult, DispatchResult, EsmtpService, MailDispatch,
        MailGuard, MailSetup, SessionInfo, StartMailRequest, StartMailResult, Transaction,
    },
};

#[derive(Default, Debug)]
pub struct Builder {
    pub id: String,
    pub dispatch: Vec<Box<dyn MailDispatch + Sync + Send + 'static>>,
    pub guard: Vec<Box<dyn MailGuard + Sync + Send + 'static>>,
    pub esmtp: Vec<Box<dyn EsmtpService + Sync + Send + 'static>>,
}

impl Builder {
    pub fn using(mut self, setup: impl MailSetup) -> Self {
        trace!("Builder {} using setup {:?}", self.id, setup);
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

impl MailGuard for Builder {
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

impl EsmtpService for Builder {
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
