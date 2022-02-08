use crate::common::*;
use crate::io::tls::MayBeTls;
use crate::mail::SvcBunch;
use crate::smtp::SmtpContext;
use crate::config::{Component, ComposableComponent, MultiComponent};
use std::ops::Deref;

/**
The service which implements this trait delivers ESMTP extensions.

```
use samotop_core::common::*;
use samotop_core::smtp::*;
use samotop_core::io::tls::MayBeTls;
use std::time::Duration;

/// This mail service can handle 8-bit MIME
#[derive(Clone, Debug)]
pub struct EnableEightBit;

impl SessionService for EnableEightBit
{
    fn prepare_session<'a, 'i, 's, 'f>(
        &'a self,
        _io: &'i mut Box<dyn MayBeTls>,
        state: &'s mut SmtpContext,
    ) -> S1Fut<'f, ()>
    where
        'a: 'f,
        'i: 'f,
        's: 'f
    {
        Box::pin(async move {
            state.session
                .extensions
                .enable(&extension::EIGHTBITMIME);
        })
    }
}
```
*/
pub trait SessionSetup: fmt::Debug {
    fn setup_session<'a, 'i, 's, 'f>(
        &'a self,
        io: &'i mut Box<dyn MayBeTls>,
        state: &'s mut SmtpContext,
    ) -> S1Fut<'f, ()>
    where
        'a: 'f,
        'i: 'f,
        's: 'f;
}
pub struct SessionSetupService {}
impl Component for SessionSetupService {
    type Target = Arc<dyn SessionSetup + Send + Sync>;
}
impl MultiComponent for SessionSetupService {}
impl ComposableComponent for SessionSetupService {
    fn compose<'a, I>(options: I) -> Option<Self::Target>
    where
        I: Iterator<Item = &'a Self::Target> + 'a,
        Self::Target: Clone + 'a,
    {
        Some(Arc::new(SvcBunch(options.cloned().collect())))
    }
}

impl<S: SessionSetup + ?Sized, T: Deref<Target = S>> SessionSetup for T
where
    T: fmt::Debug + Send + Sync,
    S: Sync,
{
    fn setup_session<'a, 'i, 's, 'f>(
        &'a self,
        io: &'i mut Box<dyn MayBeTls>,
        state: &'s mut SmtpContext,
    ) -> S1Fut<'f, ()>
    where
        'a: 'f,
        'i: 'f,
        's: 'f,
    {
        Box::pin(async move { S::setup_session(Deref::deref(self), io, state).await })
    }
}

impl SessionSetup for SvcBunch<<SessionSetupService as Component>::Target> {
    fn setup_session<'a, 'i, 's, 'f>(
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
            for svc in self.0.iter() {
                trace!("setup_session calling {:?}", svc);
                svc.setup_session(io, state).await;
            }
        })
    }
}
impl SessionSetup for Dummy {
    fn setup_session<'a, 'i, 's, 'f>(
        &'a self,
        _io: &'i mut Box<dyn MayBeTls>,
        _state: &'s mut SmtpContext,
    ) -> S1Fut<'f, ()>
    where
        'a: 'f,
        'i: 'f,
        's: 'f,
    {
        Box::pin(ready(()))
    }
}
