use crate::{common::*, parser::Parser};

pub trait ParserProvider: fmt::Debug {
    fn get_parser_for_data(&self) -> Box<dyn Parser + Sync + Send>;
    fn get_parser_for_commands(&self) -> Box<dyn Parser + Sync + Send>;
}
impl<T> ParserProvider for Arc<T>
where
    T: ParserProvider,
{
    fn get_parser_for_data(&self) -> Box<dyn Parser + Sync + Send> {
        T::get_parser_for_data(self)
    }
    fn get_parser_for_commands(&self) -> Box<dyn Parser + Sync + Send> {
        T::get_parser_for_commands(self)
    }
}
