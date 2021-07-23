use std::ops::Deref;
use std::time::Duration;

use crate::common::*;
use crate::io::tls::MayBeTls;
use crate::smtp::SmtpState;

/**
The service which implements this trait delivers ESMTP extensions.

```
use samotop_core::common::S1Fut;
use samotop_core::smtp::*;
use samotop_core::io::tls::MayBeTls;
use std::time::Duration;

/// This mail service canhabdle 8-bit MIME
#[derive(Clone, Debug)]
pub struct EnableEightBit;

impl EsmtpService for EnableEightBit
{
    fn read_timeout(&self) -> Option<Duration> { None }
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
pub trait EsmtpService: fmt::Debug {
    fn read_timeout(&self) -> Option<Duration>;
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

impl<S: EsmtpService + ?Sized, T: Deref<Target = S>> EsmtpService for T
where
    T: fmt::Debug + Send + Sync,
    S: Sync,
{
    fn read_timeout(&self) -> Option<Duration> {
        S::read_timeout(&self)
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
        Box::pin(async move { S::prepare_session(Deref::deref(self), io, state).await })
    }
}

impl EsmtpService for Dummy {
    fn read_timeout(&self) -> Option<Duration> {
        None
    }

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
