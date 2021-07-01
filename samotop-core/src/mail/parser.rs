use crate::{common::*, smtp::Interpret};

pub trait ParserProvider: fmt::Debug {
    fn get_interpretter(&self) -> Box<dyn Interpret + Sync + Send>;
}
impl<T> ParserProvider for Arc<T>
where
    T: ParserProvider,
{
    fn get_interpretter(&self) -> Box<dyn Interpret + Sync + Send> {
        T::get_interpretter(self)
    }
}
