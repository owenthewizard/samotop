#[macro_use]
extern crate log;

mod dirmail;
mod lmtp;

pub use samotop_delivery::smtp::net;

pub struct Config<Variant> {
    variant: Variant,
}

pub mod variant {
    pub use super::dirmail::Dir;
    pub use super::lmtp::LmtpDispatch;
}
