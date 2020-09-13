/*!
You can run your own privacy focussed, resource efficient mail server. [Samotop docker image](https://hub.docker.com/r/brightopen/samotop) is available for your convenience.

# Status

## Mail delivery agent (MDA)

- [x] The server will receive mail and write it to a given maildir folder. Another program can pick the folder andprocess it further.
- [x] STARTTLS can be configured if you provide a cert and identity file.
- [ ] Antispam features:
  - [x] SPF - refuse mail with failing SPF check
- [ ] Privacy features

## Mail transfer agent (MTA)

- [ ] Mail relay

# Installation

a) Using cargo:
   ```bash
   cargo install samotop-server
   ```
b) Using docker:
   ```bash
   docker pull brightopen/samotop
   ```

# Usage

a) locally, run `samotop-server --help` for command-line reference.
b) in docker, run `docker run --rm -ti samotop`

Both should produce a usage information not too different from this:

    samotop 1.0.1

    USAGE:
        samotop-server [FLAGS] [OPTIONS] --cert-file <cert file path> --identity-file <identity file path>

    FLAGS:
        -h, --help       Prints help information
            --no-tls     Disable TLS suport
        -V, --version    Prints version information

    OPTIONS:
        -n, --name <SMTP service name>              Use the given name in SMTP greetings, or if absent, use hostname
        -b, --base-dir <base dir path>              What is the base dir for other relative paths? [default: .]
        -c, --cert-file <cert file path>            Use this cert file for TLS. If a relative path is given, it will be
                                                    relative to base-dir
        -i, --identity-file <identity file path>    Use this identity file for TLS. If a relative path is given, it will be
                                                    relative to base-dir
        -m, --mail-dir <mail dir path>              Where to store incoming mail? If a relative path is given, it will be
                                                    relative to base-dir [default: inmail]
        -p, --port <port>...                        SMTP server address:port, such as 127.0.0.1:25 or localhost:12345. The
                                                    option can be set multiple times and the server will start on all given
                                                    ports. If no ports are given, the default is to start on localhost:25

# TLS

Generate a cert and ID with openssl:
```bash
openssl req -new -newkey rsa:4096 -x509 -sha256 -days 365 -nodes -out Samotop.crt -keyout Samotop.key
```

Test STARTTLS:
```bash
openssl s_client -connect localhost:25 -starttls smtp
```

Debug with STARTTLS:
```bash
openssl s_client -connect localhost:25 -debug -starttls smtp
```
 */

use async_std::fs::File;
use async_std::io::ReadExt;
use async_std::task;
use async_tls::TlsAcceptor;
use log::*;
use rustls::ServerConfig;
use samotop::server::Server;
use samotop::service::mail::default::DefaultMailService;
use samotop::service::mail::dirmail::Config as DirMailConfig;
use samotop::service::mail::MailServiceBuilder;
use samotop::service::parser::SmtpParser;
use samotop::service::tcp::tls::provide_rustls;
use samotop::service::tcp::{smtp::SmtpService, tls::TlsEnabled};
use std::path::{Path, PathBuf};
use std::sync::Arc;
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
    let tls_acceptor =
        tls_config.map(|cfg| provide_rustls(TlsAcceptor::from(std::sync::Arc::new(cfg))));
    let mail_service = DefaultMailService::new(setup.get_my_name())
        .using(DirMailConfig::new(setup.get_mail_dir()))
        .using(samotop::service::mail::spf::provide_viaspf());
    let smtp_service = SmtpService::new(Arc::new(mail_service), SmtpParser);
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

        if opt.no_tls {
            return Ok(None);
        }

        let key = {
            let id_path = self.absolute_path(
                &opt.identity_file
                    .as_ref()
                    .expect("identity-file must be set unless --no-tls"),
            );
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
            let cert_path = self.absolute_path(
                &opt.cert_file
                    .as_ref()
                    .expect("cert-file must be set unless --no-tls"),
            );
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
