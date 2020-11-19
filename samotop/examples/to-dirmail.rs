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

find /tmp/samotop/spool/
```
 */

use async_std::task;
use samotop::{
    io::{smtp::SmtpService, tls::TlsEnabled},
    mail::{Builder, DefaultMailService, Dir},
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
    let mail_service = Builder::default()
        .using(DefaultMailService::new("test-samotop".to_owned()))
        .using(Dir::new("/tmp/samotop/spool/".into())?);
    let smtp_service = SmtpService::new(Arc::new(mail_service), SmtpParser);
    let tls_smtp_service = TlsEnabled::disabled(smtp_service);

    Server::on("localhost:2525").serve(tls_smtp_service).await
}
