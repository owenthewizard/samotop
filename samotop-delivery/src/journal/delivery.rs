use super::JournalTransport;
use crate::dispatch::DispatchMail;
use samotop_core::{common::*, mail::*};
use std::path::PathBuf;

#[derive(Debug)]
pub struct Journal {
    pub path: PathBuf,
}

impl Journal {
    pub fn new(path: PathBuf) -> Result<Journal> {
        Ok(Journal { path })
    }
}

impl MailSetup for Journal {
    fn setup(self, builder: &mut Builder) {
        let transport = JournalTransport::from_dir(self.path);
        builder
            .dispatch
            .insert(0, Box::new(DispatchMail::new(transport)))
    }
}
