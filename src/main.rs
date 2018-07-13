extern crate env_logger;
extern crate samotop;
extern crate tokio;
#[macro_use]
extern crate structopt;

use std::path::PathBuf;
use structopt::StructOpt;

fn main() {
    env_logger::init();

    let opt = Opt::from_args();

    // TODO: Advertise STARTTLS in proper ESMTP way
    // TODO: let the user chose TLS mode and be more verbose about failures
    // TLS configuration, letting the user chose the identity file
    let mut tlsconf = samotop::model::controll::TlsConfig::default();
    tlsconf.id.file = PathBuf::from(opt.identity_file);

    // Mail service, use a given name or default to host name
    let mail = match opt.name {
        None => samotop::service::mail::ConsoleMail::default(),
        Some(name) => samotop::service::mail::ConsoleMail::new(name),
    };

    // Tcp service
    let tcp = samotop::service::tcp::SamotopService::new(
        samotop::service::session::StatefulSessionService::new(mail),
        tlsconf.check_identity(),
    );

    // Build the server task
    let task = samotop::builder().with(tcp).on_all(opt.ports).build_task();

    // Run the server
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

    /// If TLS feature is enabled, use this identity file
    #[structopt(short = "i", long = "identity-file", name = "file", default_value = "Samotop.pfx")]
    identity_file: String,

    /// Use the given name in SMTP greetings, or if absent, use hostname
    #[structopt(short = "n", long = "name", name = "SMTP name")]
    name: Option<String>,
}
