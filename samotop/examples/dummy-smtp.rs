//! Demonstrating the simplest use of an SMTP server.
//!
//! Run it:
//! ```
//! RUST_LOG=trace cargo run --example dummy-smtp
//! ```
//!
//! Check the log output to learn where the server is listening and connect to it.
//! (note, the port is allocated dynamically)
//! ```
//! nc 127.0.0.1 25252
//! ```
//!
//! It will print the local and remote endponits to the log and end the connection.
//!

extern crate async_std;
extern crate env_logger;
extern crate samotop;

use samotop::io::smtp::SmtpService;
use samotop::io::tls::TlsEnabled;
use samotop::parser::SmtpParser;
use samotop::server::Server;
use samotop::smtp::dummy::DummySessionService;

fn main() {
    println!("Run this with RUST_LOG=info to see the port listened on");
    env_logger::init();
    let dummy = DummySessionService::new("dummy".to_owned());
    let svc = SmtpService::new(dummy, SmtpParser);
    let svc = TlsEnabled::disabled(svc);
    let srv = Server::on("localhost:0").serve(svc);
    async_std::task::block_on(srv).unwrap()
}
