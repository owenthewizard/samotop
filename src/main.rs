extern crate env_logger;
extern crate samotop;
extern crate tokio;
#[macro_use]
extern crate structopt;

use structopt::StructOpt;

fn main() {
    env_logger::init();

    let opt = Opt::from_args();
    let mail = samotop::service::mail::ConsoleMail::new("MySamotop");
    let sess = samotop::service::session::StatefulSessionService::new(mail);
    let svc = samotop::service::tcp::next2::SamotopService::new(sess);
    let task = samotop::server::SamotopBuilder::new("localhost:12345", svc)
        .on_all(opt.ports)
        .as_task_next();
    tokio::run(task);
    /*
    //SamotopService is the default, but you can set your own name here.
    let svc = samotop::service::tcp::default()
        .serve(samotop::service::mail::ConsoleMail::new("MySamotop"));

    tokio::run(samotop::builder().with(svc).on_all(opt.ports).as_task());
    */
}

#[derive(StructOpt, Debug)]
#[structopt(name = "samotop")]
struct Opt {
    /// SMTP server address:port
    #[structopt(short = "p", long = "port")]
    ports: Vec<String>,
}
