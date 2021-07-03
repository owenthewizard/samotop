use crate::dir::MaildirTransport;
use crate::dispatch::DispatchMail;
use samotop_core::{common::*, mail::*};
use std::path::PathBuf;

#[derive(Debug)]
pub struct Dir {
    pub path: PathBuf,
}

impl Dir {
    pub fn new(path: PathBuf) -> Result<Dir> {
        Ok(Dir { path })
    }
}

impl MailSetup for Dir {
    fn setup(self, config: &mut Configuration) {
        let transport = MaildirTransport::new(self.path);
        config
            .dispatch
            .insert(0, Box::new(DispatchMail::new(transport)))
    }
}
