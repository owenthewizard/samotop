pub mod mail;
pub mod session;
pub mod tcp;

#[derive(Clone)]
pub struct Provider<T>(pub T);