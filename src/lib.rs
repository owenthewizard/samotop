#[macro_use]
extern crate log;
extern crate env_logger;
extern crate regex;
extern crate bytes;
#[macro_use]
extern crate futures;
extern crate tokio;
extern crate tokio_codec;
extern crate hostname;

pub mod model;
pub mod protocol;
pub mod server;
pub mod service;
pub mod grammar;
pub mod util;

use server::builder::Samotop;
use service::echo::EchoService;


pub static START: Samotop<EchoService> = Samotop {
    default_port: "localhost:25",
    default_service: EchoService,
};


