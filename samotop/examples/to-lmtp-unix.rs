/*!
Example of delivering to LMTP over unix socket (dovecot).
Maps recipients to local users per domain.

## Testing

```
curl -v --url 'smtp://localhost:2525' \
--mail-from from@spf.org \
--mail-rcpt to@mikesh.info \
--upload-file - <<EOF
From: Moohoo <moo@hoo.com>
To: Yeeehaw <ye@haw.com>
Subject: Try me

xoxo
EOF
```
 */

use async_std::task;
use regex::Regex;
use samotop::{
    io::{
        client::{tls::NoTls, UnixConnector},
        smtp::SmtpService,
        tls::TlsEnabled,
    },
    mail::{Builder, DefaultMailService, LmtpDispatch, Mapper},
    parser::SmtpParser,
    server::Server,
};
use std::sync::Arc;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
    env_logger::init();
    task::block_on(main_fut())
}

async fn main_fut() -> Result<()> {
    let rcpt_map = Mapper::new(vec![
        (Regex::new(".*@(.*)")?, "$1@localhost".to_owned()), // use domain as a user name (all domain basket) anyone@example.org => example.org@localhost
        (Regex::new("[^@a-zA-Z0-9]+")?, "-".to_owned()), // sanitize the user name example.org@localhost => example-org@localhost
    ]);
    let lmtp_connector: UnixConnector<NoTls> = UnixConnector::default();
    let mail_service = Builder::default()
        .using(DefaultMailService::new("test-samotop".to_owned()))
        .using(LmtpDispatch::new("/var/run/dovecot/lmtp".to_owned(), lmtp_connector)?.reuse(0))
        .using(rcpt_map);
    let smtp_service = SmtpService::new(Arc::new(mail_service), SmtpParser);
    let tls_smtp_service = TlsEnabled::disabled(smtp_service);

    Server::on("localhost:2525").serve(tls_smtp_service).await
}