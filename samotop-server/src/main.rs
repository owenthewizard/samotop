/*!
You can run your own privacy focussed, resource efficient mail server. [Samotop docker image](https://hub.docker.com/r/brightopen/samotop) is available for your convenience.

# Status

## Mail delivery agent (MDA)

- [x] The server will receive mail and write it to a given maildir folder. Another program can pick the folder andprocess it further.
- [x] STARTTLS can be configured if you provide a cert and identity file.
- [ ] Antispam features:
       - [x] SPF
- [ ] Privacy features

## Mail transfer agent (MTA)

[ ] Mail relay

# Usage

run `samotop --help` for command-line reference.

# TLS

Generate acert and ID with openssl:
```
openssl req -new -newkey rsa:4096 -x509 -sha256 -days 365 -nodes -out Samotop.crt -keyout Samotop.key
```

Test STARTTLS:
```
openssl s_client -connect localhost:25 -starttls smtp
```

Debug with STARTTLS:
```
openssl s_client -connect localhost:25 -debug -starttls smtp
```
 */

use async_std::io::ReadExt;
use async_std::task;
use async_tls::TlsAcceptor;
use log::*;
use rustls::ServerConfig;
use samotop::server::Server;
use samotop::service::mail::dirmail::SimpleDirMail;
use samotop::service::mail::CompositeMailService;
use samotop::service::session::StatefulSessionService;
use samotop::service::tcp::{SmtpService, TlsEnabled};
use std::path::{Path, PathBuf};
use structopt::StructOpt;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
    env_logger::init();
    task::block_on(main_fut())
}

async fn main_fut() -> Result<()> {
    let setup = Setup::new();

    let ports = setup.get_service_ports();
    let tls_config = setup.get_tls_config().await?;
    let tls_acceptor = tls_config.map(|cfg| TlsAcceptor::from(std::sync::Arc::new(cfg)));
    let mail_service = CompositeMailService::default()
        .with_name(setup.get_my_name())
        .using(SimpleDirMail::new(setup.get_mail_dir()))
        .using(samotop::service::mail::spf::Config::default());
    let session_service = StatefulSessionService::new(mail_service);
    let smtp_service = SmtpService::new(session_service);
    let tls_smtp_service = TlsEnabled::new(smtp_service, tls_acceptor);

    info!("I am {}", setup.get_my_name());
    Server::on_all(ports).serve(tls_smtp_service).await
}

pub struct Setup {
    opt: Opt,
}

impl Setup {
    pub fn new() -> Setup {
        Setup {
            opt: Opt::from_args(),
        }
    }

    pub async fn get_tls_config(&self) -> Result<Option<ServerConfig>> {
        let opt = &self.opt;

        if opt.identity_file.is_empty() {
            return Ok(None);
        }

        let id_path = self.absolute_path(&opt.identity_file);
        let cert_path = self.absolute_path(&opt.cert_file);

        let mut idfile = async_std::fs::File::open(id_path).await?;
        let mut certfile = async_std::fs::File::open(cert_path).await?;

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
    pub fn get_service_ports(&self) -> Vec<String> {
        if self.opt.ports.is_empty() {
            vec!["localhost:25".to_owned()]
        } else {
            self.opt.ports.iter().map(|s| s.clone()).collect()
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
    /// start on localhost:25
    #[structopt(short = "p", long = "port", name = "port")]
    ports: Vec<String>,

    /// Use this identity file for TLS. If empty, TLS will be disabled.
    /// If a relative path is given, it will be relative to base-dir
    #[structopt(
        short = "i",
        long = "identity-file",
        name = "identity file path",
        default_value = "Samotop.key"
    )]
    identity_file: String,

    /// Use this cert file for TLS.
    /// If a relative path is given, it will be relative to base-dir
    #[structopt(
        short = "c",
        long = "cert-file",
        name = "cert file path",
        default_value = "Samotop.crt"
    )]
    cert_file: String,

    /// Use the given name in SMTP greetings, or if absent, use hostname
    #[structopt(short = "n", long = "name", name = "SMTP service name")]
    name: Option<String>,

    /// Where to store incoming mail?
    /// If a relative path is given, it will be relative to base-dir
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
        default_value = "/var/lib/samotop"
    )]
    base_dir: PathBuf,
}
