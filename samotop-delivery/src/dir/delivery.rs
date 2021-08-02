use crate::dir::MaildirTransport;
use crate::dispatch::DispatchMail;
use samotop_core::{common::*, mail::*};
use std::path::PathBuf;

/// MailSetup that adds a mail dir dispatch.
///
/// E-mails are stored in the given folder according to MailDir standard.
#[derive(Debug)]
pub struct MailDir {
    pub path: PathBuf,
}

impl MailDir {
    pub fn new(path: PathBuf) -> Result<MailDir> {
        Ok(MailDir { path })
    }
}

impl<T: AcceptsDispatch> MailSetup<T> for MailDir {
    fn setup(self, config: &mut T) {
        let transport = MaildirTransport::new(self.path);
        config.add_last_dispatch(DispatchMail::new(transport))
    }
}
