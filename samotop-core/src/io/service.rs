use std::fmt::Debug;

use crate::{
    common::*,
    io::ConnectionInfo,
    server::Session,
    store::{Component, ComposableComponent, MultiComponent},
};

/**
An object implementing this trait handles TCP connections in a `Future`.

The caller would ask the service to `handle()` the anaccepted io stream and connection
(most likely a `TcpStream`, but abstracted for testability),
then poll the returned future to completion. The service may chose
to block on the call, effectively preventing concurrent connections,
or better to spawn into background and return immediately so that other
connections can be received. It could also push back on accepting
more connections if it notices too many connections or another resource
shortage by blocking on the `handle()` call.

The `SmtpService` and `DummyService` implement this trait.
*/
pub trait Handler: Debug {
    fn handle<'s, 'a, 'f>(&'s self, session: &'a mut Session) -> S2Fut<'f, Result<()>>
    where
        's: 'f,
        'a: 'f;
}
pub struct HandlerService {}
impl Component for HandlerService {
    type Target = Arc<dyn Handler + Send + Sync>;
}
impl MultiComponent for HandlerService {}
impl ComposableComponent for HandlerService {
    fn compose<'a, I>(options: I) -> Self::Target
    where
        I: Iterator<Item = &'a Self::Target> + 'a,
        Self::Target: Clone + 'a,
    {
        Arc::new(options.cloned().collect::<Vec<_>>())
    }
}

impl Handler for Vec<<HandlerService as Component>::Target> {
    fn handle<'s, 'a, 'f>(&'s self, session: &'a mut Session) -> S2Fut<'f, Result<()>>
    where
        's: 'f,
        'a: 'f,
    {
        Box::pin(async move {
            for h in self {
                h.handle(session).await?;
            }
            FallBack.handle(session).await
        })
    }
}

impl Handler for FallBack {
    fn handle<'s, 'a, 'f>(
        &'s self,
        session: &'a mut crate::server::Session,
    ) -> S2Fut<'f, Result<()>>
    where
        's: 'f,
        'a: 'f,
    {
        info!(
            "Handled connection {:?}",
            session.store.get_ref::<ConnectionInfo>()
        );
        Box::pin(ready(Ok(())))
    }
}
