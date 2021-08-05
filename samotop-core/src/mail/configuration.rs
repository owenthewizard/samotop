use crate::{common::*, io::tls::MayBeTls, mail::*, smtp::*};

/// Service builder configuration passed to `MailSetup`
#[derive(Debug)]
pub struct Configuration {
    /// ID used for identifying this instance in logs
    id: String,
    dispatch: Vec<Box<dyn MailDispatch + Sync + Send + 'static>>,
    guard: Vec<Box<dyn MailGuard + Sync + Send + 'static>>,
    session: Vec<Box<dyn SessionService + Sync + Send + 'static>>,
    interpret: Vec<Box<dyn Interpret + Sync + Send + 'static>>,
}
impl Default for Configuration {
    fn default() -> Self {
        Self {
            id: Identify::now().to_string(),
            dispatch: Default::default(),
            guard: Default::default(),
            session: Default::default(),
            interpret: Default::default(),
        }
    }
}
impl Configuration {
    pub fn into_service(self, driver: impl Drive + Sync + Send + 'static) -> Service {
        let Configuration {
            id,
            session,
            guard,
            dispatch,
            interpret,
        } = self;
        Service::new(
            driver,
            Interpretter::new(interpret),
            SvcBunch {
                id: id.clone(),
                items: session,
            },
            SvcBunch {
                id: id.clone(),
                items: guard,
            },
            SvcBunch {
                id,
                items: dispatch,
            },
        )
    }
}
impl HasId for Configuration {
    fn id(&self) -> &str {
        self.id.as_str()
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
        let session = wrap(Box::new(
            SvcBunch::<Box<dyn SessionService + Sync + Send>> {
                id: format!("({})", self.id),
                items,
            },
        ));
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
        let guard = wrap(Box::new(SvcBunch {
            id: format!("({})", self.id),
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
        let dispatch = wrap(Box::new(SvcBunch {
            id: format!("({})", self.id),
            items,
        }) as Box<dyn MailDispatch + Sync + Send>);
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

#[derive(Debug)]
struct SvcBunch<T> {
    id: String,
    items: Vec<T>,
}

impl SessionService for SvcBunch<Box<dyn SessionService + Sync + Send>> {
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
        Box::pin(async move {
            trace!(
                "SessionService {} with {} children prepare_session conn id {}",
                self.id,
                self.items.len(),
                state.session.connection.id
            );

            for svc in self.items.iter() {
                trace!(
                    "SessionService {} prepare_session calling {:?}",
                    self.id,
                    svc
                );
                svc.prepare_session(io, state).await;
            }
        })
    }
}

impl MailGuard for SvcBunch<Box<dyn MailGuard + Sync + Send>> {
    fn add_recipient<'a, 's, 'f>(
        &'a self,
        session: &'s mut SmtpSession,
        mut rcpt: Recipient,
    ) -> S2Fut<'f, AddRecipientResult>
    where
        'a: 'f,
        's: 'f,
    {
        trace!(
            "Guard {} with {} guards add_recipient {:?} to mail id {}",
            self.id,
            self.items.len(),
            rcpt,
            session.transaction.id
        );
        let fut = async move {
            for guard in self.items.iter() {
                trace!("Guard {} add_recipient calling {:?}", self.id, guard);
                match guard.add_recipient(session, rcpt).await {
                    AddRecipientResult::Inconclusive(r) => rcpt = r,
                    otherwise => return otherwise,
                }
            }
            Dummy.add_recipient(session, rcpt).await
        };
        Box::pin(fut)
    }

    fn start_mail<'a, 's, 'f>(&'a self, session: &'s mut SmtpSession) -> S2Fut<'f, StartMailResult>
    where
        'a: 'f,
        's: 'f,
    {
        trace!(
            "Guard {} with {} guards start_mail id {}",
            self.id,
            self.items.len(),
            session.transaction.id
        );
        let fut = async move {
            for guard in self.items.iter() {
                trace!("Guard {} start_mail calling {:?}", self.id, guard);
                match guard.start_mail(session).await {
                    StartMailResult::Accepted => {}
                    otherwise => return otherwise,
                }
            }
            Dummy.start_mail(session).await
        };
        Box::pin(fut)
    }
}

impl MailDispatch for SvcBunch<Box<dyn MailDispatch + Sync + Send>> {
    fn open_mail_body<'a, 's, 'f>(
        &'a self,
        session: &'s mut SmtpSession,
    ) -> S1Fut<'f, DispatchResult>
    where
        'a: 'f,
        's: 'f,
    {
        trace!(
            "Dispatch {} with {} dispatchers sending mail id {} from session {}",
            self.id,
            self.items.len(),
            session.transaction.id,
            session
        );
        let fut = async move {
            for disp in self.items.iter() {
                trace!("Dispatch {} send_mail calling {:?}", self.id, disp);
                disp.open_mail_body(session).await?;
            }
            Dummy.open_mail_body(session).await
        };
        Box::pin(fut)
    }
}
