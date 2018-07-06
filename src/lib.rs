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

use service::samotop::SamotopService;

pub fn builder() -> server::builder::SamotopBuilder<SamotopService> {
    server::builder::SamotopBuilder::new("localhost:25", SamotopService::new("Samotop"))
}

pub fn next() -> server::builder2::SamotopBuilder<SamotopService> {
    server::builder2::SamotopBuilder::new("localhost:25", SamotopService::new("Samotop"))
}
