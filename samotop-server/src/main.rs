/*!
You can run your own privacy focussed, resource efficient mail server. [Samotop docker image](https://hub.docker.com/r/brightopen/samotop) is available for your convenience.

# Status

## General

- [x] Tiny docker image - only contains statically compiled samotop and openssl, no OS clutter.

## Common (MDA/MTA/MSA)

- [x] The server will receive mail and write it to a given maildir folder. Another program can pick the folder and process it further.
- [x] STARTTLS can be configured if you provide a cert and identity file.

## Mail delivery agent (MDA)

- [ ] Encryption at rest
- [ ] Accounts
- [ ] LMTP
- [ ] Sockets

## Mail transfer agent (MTA)

- [ ] Mail relaying
- [ ] Antispam features:
  - [x] SPF - refuse mail with failing SPF check
  - [ ] Greylisting

## Mail submission agent (MSA)

- [ ] Authentication

# Installation

- Using cargo:
   ```bash
   cargo install samotop-server
   ```
- Using docker:
   ```bash
   docker pull brightopen/samotop
   ```

# Usage

- locally, run `samotop-server --help` for command-line reference.
- in docker, run `docker run --rm -ti samotop`

Both should produce a usage information not too different from this:

    samotop 1.2.0

    USAGE:
        samotop-server [FLAGS] [OPTIONS] --cert-file <cert file path> --identity-file <identity file path>

    FLAGS:
        -h, --help       Prints help information
            --no-tls     Disable TLS suport
        -V, --version    Prints version information

    OPTIONS:
        -n, --name <SMTP service name>              Use the given name in SMTP greetings, or if absent, use hostname
        -b, --base-dir <base dir path>              What is the base dir for other relative paths? [default: .]
        -c, --cert-file <cert file path>            Use this cert file for TLS. Disabled with --no-tls. If a relative path
                                                    is given, it will be relative to base-dir
            --banner_delay <delay>                  Should we enforce prudent banner deleay? Delay is in miliseconds
        -i, --identity-file <identity file path>    Use this identity file for TLS. Disabled with --no-tls. If a relative
                                                    path is given, it will be relative to base-dir
        -m, --mail-dir <mail dir path>              Where to store incoming mail? If a relative path is given, it will be
                                                    relative to base-dir [default: inmail]
        -p, --port <port>...                        SMTP server address:port, such as 127.0.0.1:25 or localhost:12345. The
                                                    option can be set multiple times and the server will start on all given
                                                    ports. If no ports are given, the default is to start on localhost:25
            --command_timeout <timeout>             Should we enforce prudent command timeout? Timeout is in miliseconds

# TLS

You can run these openssl commands in docker as well.
This will run an openssl with the current folder mounted under /data and that is also the work dir:
```
docker run --rm -ti -v "$PWD:/data/" -w "/data/" --entrypoint openssl samotop help
```

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

## Other useful hints for TLS

For native-tls, you'd convert to pfx:
```bash
openssl pkcs12 -export -out Samotop.pfx -inkey Samotop.key -in Samotop.crt
```

Extracting pub key from cert:
```bash
openssl x509 -pubkey -noout -in Samotop.crt  > Samotop.pem
```

 */

#[macro_use]
extern crate log;

use async_std::fs::File;
use async_std::io::ReadExt;
use async_std::task;
use async_tls::TlsAcceptor;
use clap::Parser;
use rustls::ServerConfig;
use samotop::io::tls::RustlsProvider;
use samotop::mail::spf::Spf;
use samotop::mail::{Builder, DebugService, MailDir, Name};
use samotop::server::TcpServer;
use samotop::smtp::{Esmtp, EsmtpStartTls, Prudence, SmtpParser};
use std::path::{Path, PathBuf};
use std::time::Duration;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
    env_logger::init();
    task::block_on(main_fut())
}

async fn main_fut() -> Result<()> {
    let setup = Setup::from_args();

    let mut service = Builder
        + Name::new(setup.name())
        + DebugService::default()
        + Esmtp.with(SmtpParser)
        + setup.prudence()
        + Spf
        + MailDir::new(setup.mail_dir())?;

    if let Some(cfg) = setup.tls_config().await? {
        service += EsmtpStartTls.with(SmtpParser, RustlsProvider::from(TlsAcceptor::from(cfg)));
    }

    TcpServer::on_all(setup.ports())
        .serve(service.build())
        .await
}

pub struct Setup {
    opt: Opt,
}

impl Setup {
    pub fn from_args() -> Setup {
        Setup { opt: Opt::parse() }
    }

    pub fn prudence(&self) -> Prudence {
        let mut prudence = Prudence::default();
        if let Some(delay) = self.opt.prudent_banner_delay {
            prudence = prudence.with_banner_delay(Duration::from_millis(delay));
        }
        if let Some(timeout) = self.opt.prudent_command_timeout {
            prudence = prudence.with_read_timeout(Duration::from_millis(timeout));
        }
        prudence
    }

    pub async fn tls_config(&self) -> Result<Option<ServerConfig>> {
        let opt = &self.opt;

        if opt.no_tls {
            return Ok(None);
        }

        let key = {
            let id_path = self.absolute_path(
                opt.identity_file
                    .as_ref()
                    .expect("identity-file must be set unless --no-tls"),
            );
            let mut idfile = File::open(&id_path)
                .await
                .map_err(|e| format!("Could not load identity: {:?}", e))?;
            let mut idbuf = vec![];
            let _ = idfile.read_to_end(&mut idbuf).await?;
            let mut idbuf = std::io::BufReader::new(&idbuf[..]);
            let keys = rustls::internal::pemfile::pkcs8_private_keys(&mut idbuf)
                .map_err(|_| "Could not load identity key".to_string())?;
            //let key = rustls::PrivateKey(idbuf);
            keys.first()
                .ok_or(format!("No private key found in {:?}", id_path))?
                .to_owned()
        };

        let certs = {
            let cert_path = self.absolute_path(
                opt.cert_file
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
    pub fn ports(&self) -> Vec<String> {
        if self.opt.ports.is_empty() {
            vec!["localhost:25".to_owned()]
        } else {
            self.opt.ports.to_vec()
        }
    }

    /// Mail service, use a given name or default to host name
    pub fn name(&self) -> String {
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

    pub fn mail_dir(&self) -> PathBuf {
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

#[derive(Parser, Debug)]
#[command(name = "samotop", version)]
struct Opt {
    /// SMTP server address:port,
    /// such as 127.0.0.1:25 or localhost:12345.
    /// The option can be set multiple times and
    /// the server will start on all given ports.
    /// If no ports are given, the default is to
    /// start on localhost:25.
    #[arg(short = 'p', long = "port", name = "port")]
    ports: Vec<String>,

    /// Disable TLS suport.
    /// It is enabled by default to reduce accidents and remind operators of misconfiguration.
    #[arg(long = "no-tls", name = "no-tls")]
    no_tls: bool,

    /// Use this identity file for TLS. Disabled with --no-tls.
    /// If a relative path is given, it will be relative to base-dir.
    #[arg(
        short = 'i',
        long = "identity-file",
        name = "identity file path",
        required_unless_present = "no-tls"
    )]
    identity_file: Option<String>,

    /// Use this cert file for TLS. Disabled with --no-tls.
    /// If a relative path is given, it will be relative to base-dir.
    #[arg(
        short = 'c',
        long = "cert-file",
        name = "cert file path",
        required_unless_present = "no-tls"
    )]
    cert_file: Option<String>,

    /// Use the given name in SMTP greetings, or if absent, use hostname.
    #[arg(short = 'n', long = "name", name = "SMTP service name")]
    name: Option<String>,

    /// Where to store incoming mail?
    /// If a relative path is given, it will be relative to base-dir.
    #[arg(
        short = 'm',
        long = "mail-dir",
        name = "mail dir path",
        default_value = "inmail"
    )]
    mail_dir: PathBuf,

    /// What is the base dir for other relative paths?
    #[arg(
        short = 'b',
        long = "base-dir",
        name = "base dir path",
        default_value = "."
    )]
    base_dir: PathBuf,

    /// Should we enforce prudent banner deleay?
    /// Delay is in miliseconds.
    #[arg(long = "banner_delay", name = "delay")]
    prudent_banner_delay: Option<u64>,

    /// Should we enforce prudent command timeout?
    /// Timeout is in miliseconds.
    #[arg(long = "command_timeout", name = "timeout")]
    prudent_command_timeout: Option<u64>,
}
