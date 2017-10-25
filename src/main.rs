extern crate env_logger;
extern crate samotop;
extern crate tokio_proto;

use tokio_proto::TcpServer;
use samotop::service::SmtpService;
use samotop::protocol::transport::SmtpProto;

/*
   For debug log try:
     RUST_LOG=samotop=trace cargo run

   To simulate hopped smtp input:
     (sleep 2;
        echo -en "helo";
        sleep 3;
        echo -en " there\r";
        sleep 5;
        echo -en "\n"
        ) | nc localhost 12345
*/

fn main() {

    env_logger::init().unwrap();

    // Specify the localhost address
    let addr = "0.0.0.0:12345".parse().unwrap();

    // The builder requires a protocol and an address
    let server = TcpServer::new(SmtpProto, addr);

    // We provide a way to *instantiate* the service for each new
    // connection; here, we just immediately return a new instance.
    server.serve(|| Ok(SmtpService::new()));
}
