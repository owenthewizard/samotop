extern crate env_logger;
extern crate samotop;
extern crate tokio;
#[macro_use]
extern crate structopt;

use structopt::StructOpt;

fn main() {
    env_logger::init();

    let opt = Opt::from_args();

    tokio::run(samotop::builder()
            //SamotopService is the default, but you can set your own name here.
            .with(samotop::service::samotop::SamotopService::new("MySamotop"))
            .on_all(opt.ports)
            .as_task());
}

#[derive(StructOpt, Debug)]
#[structopt(name = "samotop")]
struct Opt {
    /// SMTP server address:port
    #[structopt(short = "p", long = "port")]
    ports: Vec<String>,
}
