use std::ops::Deref;

use crate::{
    common::*,
    io::tls::MayBeTls,
    smtp::{Drive, Interpret},
};

pub trait DriverProvider: fmt::Debug {
    fn get_driver<'io>(&self, io: &'io mut (dyn DriverIo)) -> Box<dyn Drive + Sync + Send + 'io>;
    fn get_interpretter(&self) -> Box<dyn Interpret + Sync + Send>;
}

impl<S: DriverProvider + ?Sized, T: Deref<Target = S>> DriverProvider for T
where
    T: fmt::Debug + Send + Sync,
    S: Sync,
{
    fn get_driver<'io>(&self, io: &'io mut (dyn DriverIo)) -> Box<dyn Drive + Sync + Send + 'io> {
        S::get_driver(Deref::deref(self), io)
    }
    fn get_interpretter(&self) -> Box<dyn Interpret + Sync + Send> {
        S::get_interpretter(Deref::deref(self))
    }
}

pub trait DriverIo: MayBeTls + Read + Write + Send + Sync + Unpin {}

impl<T> DriverIo for T where T: MayBeTls + Read + Write + Send + Sync + Unpin {}
