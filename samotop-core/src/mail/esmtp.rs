use crate::common::*;

use crate::io::tls::MayBeTls;
use crate::mail::SessionInfo;

/**
The service which implements this trait delivers ESMTP extensions.

```
# use samotop_core::smtp::*;
# use samotop_core::mail::*;
# use samotop_core::io::tls::MayBeTls;
/// This mail service canhabdle 8-bit MIME
#[derive(Clone, Debug)]
pub struct EnableEightBit<T>(T);

impl<T> EsmtpService for EnableEightBit<T>
where
    T: EsmtpService,
{
    fn prepare_session(&self, io: &mut dyn MayBeTls, session: &mut SessionInfo) {
        self.0.prepare_session(io, session);
        session
            .extensions
            .enable(&extension::EIGHTBITMIME);
    }
}
```
*/
pub trait EsmtpService: fmt::Debug {
    fn prepare_session(&self, io: &mut dyn MayBeTls, session: &mut SessionInfo);
}

impl<T> EsmtpService for Arc<T>
where
    T: EsmtpService,
{
    fn prepare_session(&self, io: &mut dyn MayBeTls, session: &mut SessionInfo) {
        T::prepare_session(self, io, session)
    }
}
