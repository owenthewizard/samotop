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

impl<T: AcceptsDispatch> MailSetup<T> for Journal {
    fn setup(self, config: &mut T) {
        let transport = JournalTransport::new(self.path);
        config.add_last_dispatch(DispatchMail::new(transport))
    }
}
