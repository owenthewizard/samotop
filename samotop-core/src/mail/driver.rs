use crate::{
    common::*,
    io::tls::MayBeTls,
    smtp::{Drive, Interpret},
};

pub trait DriverProvider: fmt::Debug {
    fn get_driver<'io>(&self, io: &'io mut (dyn DriverIo)) -> Box<dyn Drive + Sync + Send + 'io>;
    fn get_interpretter(&self) -> Box<dyn Interpret + Sync + Send>;
}
impl<T> DriverProvider for Arc<T>
where
    T: DriverProvider,
{
    fn get_driver<'io>(&self, io: &'io mut (dyn DriverIo)) -> Box<dyn Drive + Sync + Send + 'io> {
        T::get_driver(self, io)
    }
    fn get_interpretter(&self) -> Box<dyn Interpret + Sync + Send> {
        T::get_interpretter(self)
    }
}

pub trait DriverIo: MayBeTls + Read + Write + Send + Sync + Unpin {}

impl<T> DriverIo for T where T: MayBeTls + Read + Write + Send + Sync + Unpin {}
