use async_std::task;
use async_tls::TlsAcceptor;
use futures::prelude::*;
use log::*;
use rustls::ServerConfig;
use samotop::server::Server;
use samotop::service::mail::ConsoleMail;
use samotop::service::session::StatefulSessionService;
use samotop::service::tcp::{SmtpService, TlsEnabled};
use structopt::StructOpt;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
    env_logger::init();
    task::block_on(main_fut())
}

async fn main_fut() -> Result<()> {
    let opt = Opt::from_args();

    let name = get_my_name(&opt);
    let ports = get_service_ports(&opt);
    let tls_config = get_tls_config(&opt).await?;
    let tls_acceptor = tls_config.map(|cfg| TlsAcceptor::from(std::sync::Arc::new(cfg)));
    let mail_service = ConsoleMail::new(name.as_str());
    let session_service = StatefulSessionService::new(mail_service);
    //let session_service = samotop::service::session::dummy::DummySessionService::new(mail_service);
    let smtp_service = SmtpService::new(session_service);
    let tls_smtp_service = TlsEnabled::new(smtp_service, tls_acceptor);

    info!("I am {}", name);
    Server::on_all(ports).serve(tls_smtp_service).await
}

async fn get_tls_config(opt: &Opt) -> Result<Option<ServerConfig>> {
    if opt.identity_file.is_empty() {
        return Ok(None);
    }
    let mut idfile = async_std::fs::File::open(&opt.identity_file).await?;
    let mut certfile = async_std::fs::File::open(&opt.cert_file).await?;

    let mut idbuf = vec![];
    let _ = idfile.read_to_end(&mut idbuf).await?;
    let mut idbuf = std::io::BufReader::new(&idbuf[..]);
    let keys = rustls::internal::pemfile::pkcs8_private_keys(&mut idbuf)
        .ok()
        .ok_or("could not read private keys")?;
    //let key = rustls::PrivateKey(idbuf);
    let key = keys.first().ok_or("no private key found")?;

    let mut certbuf = vec![];
    let _ = certfile.read_to_end(&mut certbuf).await?;
    let mut certbuf = std::io::BufReader::new(&certbuf[..]);
    let certs = rustls::internal::pemfile::certs(&mut certbuf)
        .ok()
        .ok_or("could not read certificates")?;

    let mut config = ServerConfig::new(rustls::NoClientAuth::new());
    config.set_single_cert(certs, key.to_owned())?;
    Ok(Some(config))
}

/// Get all TCP ports to serve the service on
fn get_service_ports(opt: &Opt) -> Vec<String> {
    if opt.ports.is_empty() {
        vec!["localhost:25".to_owned()]
    } else {
        opt.ports.iter().map(|s| s.clone()).collect()
    }
}

/// Mail service, use a given name or default to host name
fn get_my_name(opt: &Opt) -> String {
    match &opt.name {
        None => match hostname::get() {
            Err(e) => {
                warn!("Unable to get hostname, using default. {}", e);
                "Samotop".into()
            }
            Ok(name) => match name.into_string() {
                Err(e) => {
                    warn!("Unable to use hostname, using default. {:?}", e);
                    "Samotop".into()
                }
                Ok(name) => name,
            },
        },
        Some(name) => name.clone(),
    }
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
    #[structopt(
        short = "i",
        long = "identity-file",
        name = "idfile",
        default_value = "Samotop.key"
    )]
    identity_file: String,

    /// If TLS feature is enabled, use this identity file
    #[structopt(
        short = "c",
        long = "cert-file",
        name = "certfile",
        default_value = "Samotop.crt"
    )]
    cert_file: String,

    /// Use the given name in SMTP greetings, or if absent, use hostname
    #[structopt(short = "n", long = "name", name = "SMTP name")]
    name: Option<String>,
}
