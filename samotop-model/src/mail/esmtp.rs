use crate::common::*;

use crate::mail::SessionInfo;

/**
The service which implements this trait delivers ESMTP extensions.

```
# use samotop_model::smtp::*;
# use samotop_model::mail::*;
/// This mail service canhabdle 8-bit MIME
#[derive(Clone, Debug)]
pub struct EnableEightBit<T>(T);

impl<T> EsmtpService for EnableEightBit<T>
where
    T: EsmtpService,
{
    fn prepare_session(&self, session: &mut SessionInfo) {
        self.0.prepare_session(session);
        session
            .extensions
            .enable(&extension::EIGHTBITMIME);
    }
}
```
*/
pub trait EsmtpService: fmt::Debug {
    fn prepare_session(&self, session: &mut SessionInfo);
}

impl<T> EsmtpService for Arc<T>
where
    T: EsmtpService,
{
    fn prepare_session(&self, session: &mut SessionInfo) {
        T::prepare_session(self, session)
    }
}
