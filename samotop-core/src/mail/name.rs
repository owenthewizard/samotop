use super::{EsmtpService, MailSetup, SessionInfo};

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
    fn prepare_session(&self, session: &mut SessionInfo) {
        session.service_name = self.name.clone();
    }
}
impl MailSetup for Name {
    fn setup(self, config: &mut super::Configuration) {
        config.esmtp.insert(0, Box::new(self))
    }
}
