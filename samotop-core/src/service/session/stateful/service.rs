use super::*;
use crate::common::*;
use crate::model::Result;
use crate::service::mail::*;
use crate::service::session::*;

pub trait HandleAsStatefulService: MailService + Sized + 'static {
    /// Allows you to specify a custom handler for your mail service
    fn handle_with<F, H>(self, handler_factory: F) -> StatefulSessionService<Self, F>
    where
        F: Fn(Arc<Self>) -> H,
        H: SessionHandler,
    {
        StatefulSessionService::new(self, handler_factory)
    }
}
impl<S> HandleAsStatefulService for S where S: MailService + Sized + 'static {}

impl<S: MailService + 'static> From<S>
    for StatefulSessionService<S, Box<dyn Fn(Arc<S>) -> BasicSessionHandler<Arc<S>>>>
{
    fn from(service: S) -> Self {
        StatefulSessionService::new(service, Box::new(|svc| BasicSessionHandler::from(svc)))
    }
}

/// Allows you to configure the session handler for a given mail service
#[derive(Clone)]
pub struct StatefulSessionService<S, F> {
    mail_service: Arc<S>,
    handler_factory: F,
}

impl<S, F, H> StatefulSessionService<S, F>
where
    H: SessionHandler,
    F: Fn(Arc<S>) -> H,
    S: MailService,
{
    pub fn new(mail_service: S, handler_factory: F) -> Self {
        Self {
            mail_service: Arc::new(mail_service),
            handler_factory,
        }
    }
    /// Allows you to specify a custom handler for your mail service
    pub fn handle_with<FChange, FNew>(self, change: FChange) -> StatefulSessionService<S, FNew>
    where
        FChange: Fn(F) -> FNew,
        FNew: Fn(S) -> H,
    {
        let StatefulSessionService {
            mail_service,
            handler_factory,
        } = self;
        StatefulSessionService {
            mail_service,
            handler_factory: change(handler_factory),
        }
    }
}

impl<I, S, H, F> SessionService<I> for StatefulSessionService<S, F>
where
    I: Stream<Item = Result<ReadControl>>,
    S: MailService,
    H: SessionHandler,
    F: Fn(Arc<S>) -> H,
{
    type Session = StatefulSession<I, H>;
    type StartFuture = future::Ready<Self::Session>;
    fn start(&self, input: I) -> Self::StartFuture {
        let handler = (self.handler_factory)(self.mail_service.clone());
        future::ready(session::StatefulSession::new(input, handler))
    }
}
