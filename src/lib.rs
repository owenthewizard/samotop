#[macro_use]
extern crate log;
extern crate env_logger;
extern crate regex;
extern crate bytes;
extern crate futures;
extern crate tokio;
extern crate tokio_codec;

pub mod model;
pub mod protocol;
pub mod server;
pub mod service;
pub mod grammar;

use server::builder::Samotop;
use service::echo::EchoService;


pub static START: Samotop<EchoService> = Samotop {
    default_port: "localhost:25",
    default_service: EchoService,
};


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
