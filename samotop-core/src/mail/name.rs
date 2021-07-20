use super::{EsmtpService, MailSetup};
use crate::common::{ready, S1Fut};
use crate::io::tls::MayBeTls;
use crate::smtp::SmtpState;

/// MailSetup that uses the given service name for a session.
#[derive(Debug)]
pub struct Name {
    name: String,
}
impl Name {
    pub fn new(name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}
impl EsmtpService for Name {
    /// Use a given name as a service name in the session
    fn prepare_session<'a, 'i, 's, 'f>(
        &'a self,
        _io: &'i mut Box<dyn MayBeTls>,
        state: &'s mut SmtpState,
    ) -> S1Fut<'f, ()>
    where
        'a: 'f,
        'i: 'f,
        's: 'f,
    {
        state.session.service_name = self.name.clone();
        Box::pin(ready(()))
    }
}
impl MailSetup for Name {
    /// Add self as an ESMTP service so it can configure service name for each session
    fn setup(self, config: &mut super::Configuration) {
        config.esmtp.insert(0, Box::new(self))
    }
}
