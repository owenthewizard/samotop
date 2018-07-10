extern crate env_logger;
extern crate samotop;
extern crate tokio;
#[macro_use]
extern crate structopt;

use structopt::StructOpt;

fn main() {
    env_logger::init();

    let opt = Opt::from_args();

    let task = samotop::builder().on_all(opt.ports).as_task();

    tokio::run(task);
}

#[derive(StructOpt, Debug)]
#[structopt(name = "samotop")]
struct Opt {
    /// SMTP server address:port,
    /// such as 127.0.0.1:25 or localhost:12345.
    /// The option can be set multiple times and
    /// the server will start on all given ports.
    /// If no ports are given, the default is to 
    /// start on localhost:25
    #[structopt(short = "p", long = "port", name = "port")]
    ports: Vec<String>,
}
