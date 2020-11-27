/*!
Example of delivering nowhere except to the console output

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
use samotop::mail::Builder;
use samotop::parser::SmtpParser;
use samotop::server::TcpServer;
use samotop::{
    io::{smtp::SmtpService, tls::TlsEnabled},
    mail::NullDispatch,
};
use std::sync::Arc;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
    env_logger::init();
    task::block_on(main_fut())
}

async fn main_fut() -> Result<()> {
    let mail_service = Builder::default()
        .using(NullDispatch)
        .using(SmtpParser::default());
    let smtp_service = SmtpService::new(Arc::new(mail_service));
    let tls_smtp_service = TlsEnabled::disabled(smtp_service);
    TcpServer::on("localhost:2525")
        .serve(tls_smtp_service)
        .await
}
