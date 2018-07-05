#[macro_use]
extern crate log;
extern crate bytes;
extern crate env_logger;
extern crate regex;
#[macro_use]
extern crate futures;
extern crate hostname;
extern crate tokio;
extern crate tokio_codec;

pub mod grammar;
pub mod model;
pub mod protocol;
pub mod server;
pub mod service;
pub mod util;

use server::builder::SamotopBuilder;
use service::samotop::SamotopService;

pub fn builder() -> SamotopBuilder<SamotopService> {
    SamotopBuilder::new("localhost:25", SamotopService::new("Samotop"))
}
