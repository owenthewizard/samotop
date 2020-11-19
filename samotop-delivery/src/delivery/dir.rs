use super::dispatch::DispatchMail;
use crate::dir::FileTransport;
use samotop_model::{common::*, mail::*};
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
    fn setup(self, builder: &mut Builder) {
        let transport = FileTransport::new(self.path);
        builder
            .dispatch
            .insert(0, Box::new(DispatchMail::new(transport)))
    }
}
