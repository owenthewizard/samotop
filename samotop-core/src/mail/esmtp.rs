use crate::common::*;
use crate::io::tls::MayBeTls;
use crate::smtp::SmtpState;

/**
The service which implements this trait delivers ESMTP extensions.

```
use samotop_core::common::S1Fut;
use samotop_core::smtp::*;
use samotop_core::mail::*;
use samotop_core::io::tls::MayBeTls;

/// This mail service canhabdle 8-bit MIME
#[derive(Clone, Debug)]
pub struct EnableEightBit;

impl EsmtpService for EnableEightBit
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
pub trait EsmtpService: fmt::Debug {
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

impl<T> EsmtpService for Arc<T>
where
    T: EsmtpService,
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
        T::prepare_session(self, io, state)
    }
}
