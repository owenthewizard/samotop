extern crate samotop;
extern crate tokio_proto;

use std::io;
use tokio_proto::TcpServer;
use samotop::service::SmtpService;
use samotop::protocol::simple::SmtpProto;

fn main() {

    // Specify the localhost address
    let addr = "0.0.0.0:12345".parse().unwrap();

    // The builder requires a protocol and an address
    let server = TcpServer::new(SmtpProto, addr);

    // We provide a way to *instantiate* the service for each new
    // connection; here, we just immediately return a new instance.
    server.serve(|| Ok(SmtpService));
}
