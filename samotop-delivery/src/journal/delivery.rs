use super::JournalTransport;
use crate::dispatch::DispatchMail;
use samotop_core::mail::*;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Journal {
    pub path: PathBuf,
}

impl Journal {
    /// Creates a journal in the given folder
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}
impl Default for Journal {
    /// Creates a journal in the current folder
    fn default() -> Self {
        Self::new(".")
    }
}

impl MailSetup for Journal {
    fn setup(self, config: &mut Configuration) {
        let transport = JournalTransport::new(self.path);
        config
            .dispatch
            .insert(0, Box::new(DispatchMail::new(transport)))
    }
}
