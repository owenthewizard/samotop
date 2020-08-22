pub mod mail;
pub mod session;
pub mod tcp;
pub mod parser;

#[derive(Clone)]
pub struct Provider<T>(pub T);
