//! Demonstrating the simplest use of a TCP server.
//!
//! Run it:
//! ```
//! RUST_LOG=trace cargo run --example dummy
//! ```
//!
//! Check the log output to learn where the server is listening and connect to it.
//! (note, the port is allocated dynamically)
//! ```
//! nc 127.0.0.1:25252
//! ```
//!
//! It will print the local and remote endponits to the log and end the connection.
//!

extern crate async_std;
extern crate env_logger;
extern crate samotop;

use samotop::server::Server;
use samotop::service::tcp::DummyTcpService;

fn main() {
    env_logger::init();
    let srv = Server::on("localhost:0").serve(DummyTcpService);
    async_std::task::block_on(srv).unwrap()
}
