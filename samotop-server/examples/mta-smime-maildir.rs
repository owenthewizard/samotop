#[macro_use]
extern crate log;

use async_std::fs::File;
use async_std::io::ReadExt;
use async_std::task;
use async_tls::TlsAcceptor;
use rustls::ServerConfig;
use samotop::mail::{Builder, Dir, Name};
use samotop::parser::SmtpParser;
use samotop::server::TcpServer;
use samotop::{io::smtp::SmtpService, mail::smime::SMimeMail};
use samotop::{io::tls::RustlsProvider, mail::smime::Accounts};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use structopt::StructOpt;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
    env_logger::init();
    task::block_on(main_fut())
}

async fn main_fut() -> Result<()> {
    let setup = Setup::from_args();

    let ports = setup.get_service_ports();

    let mut mail_service = Builder::default()
        .using(Name::new(setup.get_my_name()))
        .using(Accounts::new(setup.absolute_path("accounts")))
        .using(SMimeMail::new(
            setup.get_id_file_path().expect("id file"),
            setup.get_cert_file_path().expect("cert file"),
        ))
        .using(Dir::new(setup.get_mail_dir())?)
        .using(samotop::mail::spf::provide_viaspf())
        .using(SmtpParser::default());

    if let Some(cfg) = setup.get_tls_config().await? {
        mail_service = mail_service.using(RustlsProvider::from(TlsAcceptor::from(cfg)));
    }

    let smtp_service = SmtpService::new(Arc::new(mail_service));

    info!("I am {}", setup.get_my_name());
    TcpServer::on_all(ports).serve(smtp_service).await
}

pub struct Setup {
    opt: Opt,
}

impl Setup {
    pub fn from_args() -> Setup {
        Setup {
            opt: Opt::from_args(),
        }
    }

    pub fn get_id_file_path(&self) -> Option<PathBuf> {
        let path = self.absolute_path(self.opt.identity_file.as_ref()?);
        Some(path)
    }

    pub fn get_cert_file_path(&self) -> Option<PathBuf> {
        let path = self.absolute_path(self.opt.cert_file.as_ref()?);
        Some(path)
    }

    pub async fn get_tls_config(&self) -> Result<Option<ServerConfig>> {
        let opt = &self.opt;

        if opt.no_tls {
            return Ok(None);
        }

        let key = {
            let id_path = self
                .get_id_file_path()
                .expect("identity-file must be set unless --no-tls");
            let mut idfile = File::open(&id_path)
                .await
                .map_err(|e| format!("Could not load identity: {}", e))?;
            let mut idbuf = vec![];
            let _ = idfile.read_to_end(&mut idbuf).await?;
            let mut idbuf = std::io::BufReader::new(&idbuf[..]);
            let keys = rustls::internal::pemfile::pkcs8_private_keys(&mut idbuf)
                .map_err(|_| format!("Could not load identity from {:?}", id_path))?;
            //let key = rustls::PrivateKey(idbuf);
            keys.first()
                .ok_or(format!("No private key found in {:?}", id_path))?
                .to_owned()
        };

        let certs = {
            let cert_path = self
                .get_cert_file_path()
                .expect("cert-file must be set unless --no-tls");
            let mut certfile = File::open(&cert_path)
                .await
                .map_err(|e| format!("Could not load certs: {}", e))?;
            let mut certbuf = vec![];
            let _ = certfile.read_to_end(&mut certbuf).await?;
            let mut certbuf = std::io::BufReader::new(&certbuf[..]);
            let certs = rustls::internal::pemfile::certs(&mut certbuf)
                .map_err(|_| format!("Could not load certs from {:?}", cert_path))?;
            certs
                .first()
                .ok_or(format!("No certs found in {:?}", cert_path))?;
            certs
        };

        let mut config = ServerConfig::new(rustls::NoClientAuth::new());
        config.set_single_cert(certs, key)?;
        Ok(Some(config))
    }

    /// Get all TCP ports to serve the service on
    pub fn get_service_ports(&self) -> Vec<String> {
        if self.opt.ports.is_empty() {
            vec!["localhost:25".to_owned()]
        } else {
            self.opt.ports.to_vec()
        }
    }

    /// Mail service, use a given name or default to host name
    pub fn get_my_name(&self) -> String {
        match &self.opt.name {
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

    pub fn get_mail_dir(&self) -> PathBuf {
        self.absolute_path(&self.opt.mail_dir)
    }

    fn absolute_path(&self, path: impl AsRef<Path>) -> PathBuf {
        if path.as_ref().is_absolute() {
            path.as_ref().to_owned()
        } else {
            self.opt.base_dir.join(path)
        }
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
    /// start on localhost:25.
    #[structopt(short = "p", long = "port", name = "port")]
    ports: Vec<String>,

    /// Disable TLS suport.
    /// It is enabled by default to reduce accidents and remind operators of misconfiguration.
    #[structopt(long = "no-tls")]
    no_tls: bool,

    /// Use this identity file for TLS. Disabled with --no-tls.
    /// If a relative path is given, it will be relative to base-dir.
    #[structopt(
        short = "i",
        long = "identity-file",
        name = "identity file path",
        required_unless = "no-tls"
    )]
    identity_file: Option<String>,

    /// Use this cert file for TLS. Disabled with --no-tls.
    /// If a relative path is given, it will be relative to base-dir.
    #[structopt(
        short = "c",
        long = "cert-file",
        name = "cert file path",
        required_unless = "no-tls"
    )]
    cert_file: Option<String>,

    /// Use the given name in SMTP greetings, or if absent, use hostname.
    #[structopt(short = "n", long = "name", name = "SMTP service name")]
    name: Option<String>,

    /// Where to store incoming mail?
    /// If a relative path is given, it will be relative to base-dir.
    #[structopt(
        short = "m",
        long = "mail-dir",
        name = "mail dir path",
        default_value = "inmail"
    )]
    mail_dir: PathBuf,

    /// What is the base dir for other relative paths?
    #[structopt(
        short = "b",
        long = "base-dir",
        name = "base dir path",
        default_value = "."
    )]
    base_dir: PathBuf,
}
