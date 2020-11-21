/*!
Example of delivering to LMTP over unix socket (dovecot).
Maps recipients to local users per domain.

## Testing

```
sed -e 's/$/\r/' <<EOF | curl -v --url 'smtp://localhost:2525' \
--mail-from from@spf.org \
--mail-rcpt to@mikesh.info \
--upload-file -
From: Moohoo <moo@hoo.com>
To: Yeeehaw <ye@haw.com>
Subject: Try me

.
..
xoxo
EOF

find tmp/samotop/spool/
```
 */

use async_std::task;
use samotop::{
    io::{smtp::SmtpService, tls::TlsEnabled},
    mail::{Builder, Dir},
    parser::SmtpParser,
    server::UnixServer,
};
use std::sync::Arc;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
    env_logger::init();
    task::block_on(main_fut())
}

async fn main_fut() -> Result<()> {
    let dir_service = Dir::new("tmp/samotop/spool/".into())?;
    let mail_service = Arc::new(Builder::default().using(dir_service));
    let smtp_service = SmtpService::new(mail_service, SmtpParser);
    let tls_smtp_service = TlsEnabled::disabled(smtp_service);

    UnixServer::on("local.socket").serve(tls_smtp_service).await
}
