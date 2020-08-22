pub mod io;
pub mod mail;
pub mod smtp;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;
