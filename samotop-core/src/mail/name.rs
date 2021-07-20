use crate::io::tls::MayBeTls;

use super::{EsmtpService, MailSetup, SessionInfo};

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
    fn prepare_session(&self, _io: &mut dyn MayBeTls, session: &mut SessionInfo) {
        session.service_name = self.name.clone();
    }
}
impl MailSetup for Name {
    /// Add self as an ESMTP service so it can configure service name for each session
    fn setup(self, config: &mut super::Configuration) {
        config.esmtp.insert(0, Box::new(self))
    }
}
