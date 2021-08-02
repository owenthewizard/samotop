//! Example of encription at rest using S/MIME
//!
//! The accounts folder must have a "certificate" for each recipient.
//! E-mails received over SMTP are encrypted on the fly for all recipients
//! before being passed to another `MailDispatch` - here the simple maildir
//! - but could be also the LMTP delivery or your own `MailDispatch` implementation.
//! Thus, the e-mail is never stored on disk in plaintext except perhaps through a swap file.
//!
//! The server will refuse e-mail (temporarily) for recipients who do not have a certificate.
//!
//! A specific `MailGuard` could rewrite rcpt addresses in such a way (hash) to hide the recipient's
//! e-mail address as well and still link it to a specific user who is possibly only identified
//! by having the private key to his cert.

#[macro_use]
extern crate log;

use async_std::fs::File;
use async_std::io::ReadExt;
use async_std::task;
use async_tls::TlsAcceptor;
use rustls::ServerConfig;
use samotop::{
    io::tls::RustlsProvider,
    mail::{
        smime::{Accounts, SMimeMail},
        spf::Spf,
        Builder, MailDir, Name,
    },
    server::TcpServer,
    smtp::{Esmtp, EsmtpStartTls, SmtpParser},
};
use std::path::{Path, PathBuf};
use structopt::StructOpt;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
    env_logger::init();
    task::block_on(main_fut())
}

async fn main_fut() -> Result<()> {
    let setup = Setup::from_args();

    let ports = setup.get_service_ports();

    let service = Builder
        + Name::new(setup.get_my_name())
        + Accounts::new(setup.absolute_path("accounts"))
        + SMimeMail::new(setup.get_id_file_path(), setup.get_cert_file_path())
        + MailDir::new(setup.get_mail_dir())?
        + Spf
        + Esmtp.with(SmtpParser)
        + EsmtpStartTls.with(
            SmtpParser,
            RustlsProvider::from(TlsAcceptor::from(setup.get_tls_config().await?)),
        );

    info!("I am {}", setup.get_my_name());
    TcpServer::on_all(ports).serve(service.build()).await
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

    pub fn get_id_file_path(&self) -> PathBuf {
        self.absolute_path(&self.opt.identity_file)
    }

    pub fn get_cert_file_path(&self) -> PathBuf {
        self.absolute_path(&self.opt.cert_file)
    }

    pub async fn get_tls_config(&self) -> Result<ServerConfig> {
        let key = {
            let id_path = self.get_id_file_path();
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
            let cert_path = self.get_cert_file_path();
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
        Ok(config)
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

    /// Use this identity file for TLS. Disabled with --no-tls.
    /// If a relative path is given, it will be relative to base-dir.
    #[structopt(
        short = "i",
        long = "identity-file",
        name = "identity file path",
        required = true
    )]
    identity_file: PathBuf,

    /// Use this cert file for TLS. Disabled with --no-tls.
    /// If a relative path is given, it will be relative to base-dir.
    #[structopt(
        short = "c",
        long = "cert-file",
        name = "cert file path",
        required = true
    )]
    cert_file: PathBuf,

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
