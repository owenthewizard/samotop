#[macro_use]
extern crate log;
extern crate env_logger;
extern crate samotop;
extern crate tokio_proto;
#[macro_use]
extern crate structopt;

use samotop::protocol::simple::SmtpProto;
use samotop::service::SmtpService;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::str::FromStr;
use structopt::StructOpt;
use tokio_proto::TcpServer;

#[derive(StructOpt, Debug)]
#[structopt(name = "samotop")]
struct Opt {
    /// SMTP server address
    #[structopt(short = "s", long = "server", default_value = "0.0.0.0:12345")]
    mailserver: String,

    /// Mode of operation (server or test)
    #[structopt(short = "m", long = "mode", default_value = "server")]
    mode: Mode,
}

#[derive(Debug)]
enum Mode {
    Test,
    Server,
}

impl FromStr for Mode {
    type Err = String;
    fn from_str(str: &str) -> Result<Self, Self::Err> {
        match str.to_lowercase().as_ref() {
            "test" => Ok(Mode::Test),
            "server" => Ok(Mode::Server),
            _ => Err(format!("Invalid mode: {}", str)),
        }
    }
}

fn main() {
    env_logger::init();

    let opt = Opt::from_args();
    trace!("{:?}", opt);

    match opt.mode {
        Mode::Server => {
            // Specify the localhost address
            let addr = opt.mailserver.parse().unwrap();

            // The builder requires a protocol and an address
            let server = TcpServer::new(SmtpProto, addr);

            // We provide a way to *instantiate* the service for each new
            // connection; here, we just immediately return a new instance.
            server.serve(|| Ok(SmtpService));
        }
        Mode::Test => {
            let mut stream =
                TcpStream::connect(opt.mailserver).expect("failed to connect to server");
            stream
                .set_nonblocking(true)
                .expect("set_nonblocking call failed");

            let mut input = std::io::stdin();

            let buf = &mut [0u8; 1024];

            loop {
                if let Ok(n) = input.read(&mut buf[..]) {
                    let pass = match &buf[..n] {
                        &[b'#', b'\n'] => {
                            println!("#!");
                            vec![]
                        },
                        _ => {
                            let mut v = vec![];
                            v.extend_from_slice(&buf[..n - 1]);
                            v.extend_from_slice(&b"\r\n"[..]);
                            v
                        }
                    };
                    stream.write_all(&pass).expect("could not write to stream");
                }

                while let Ok(n) = stream.read(&mut buf[..]) {
                    print!(
                        "{}",
                        String::from_utf8(buf[..n].to_vec()).expect("invalid utf-8")
                    );
                }
            }
        }
    }
}
