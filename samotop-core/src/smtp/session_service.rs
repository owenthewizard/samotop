use crate::common::*;
use crate::io::tls::MayBeTls;
use crate::smtp::SmtpState;
use std::ops::Deref;

/**
The service which implements this trait delivers ESMTP extensions.

```
use samotop_core::common::*;
use samotop_core::smtp::*;
use samotop_core::io::tls::MayBeTls;
use std::time::Duration;

/// This mail service canhabdle 8-bit MIME
#[derive(Clone, Debug)]
pub struct EnableEightBit;

impl SessionService for EnableEightBit
{
    fn prepare_session<'a, 'i, 's, 'f>(
        &'a self,
        _io: &'i mut Box<dyn MayBeTls>,
        state: &'s mut SmtpState,
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
pub trait SessionService: fmt::Debug {
    fn prepare_session<'a, 'i, 's, 'f>(
        &'a self,
        io: &'i mut Box<dyn MayBeTls>,
        state: &'s mut SmtpState,
    ) -> S1Fut<'f, ()>
    where
        'a: 'f,
        'i: 'f,
        's: 'f;
}

impl<S: SessionService + ?Sized, T: Deref<Target = S>> SessionService for T
where
    T: fmt::Debug + Send + Sync,
    S: Sync,
{
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
        Box::pin(async move { S::prepare_session(Deref::deref(self), io, state).await })
    }
}

impl SessionService for Dummy {
    fn prepare_session<'a, 'i, 's, 'f>(
        &'a self,
        _io: &'i mut Box<dyn MayBeTls>,
        _state: &'s mut SmtpState,
    ) -> S1Fut<'f, ()>
    where
        'a: 'f,
        'i: 'f,
        's: 'f,
    {
        Box::pin(ready(()))
    }
}
