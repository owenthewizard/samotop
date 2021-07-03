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

use samotop::mail::{Builder, Esmtp};
use samotop::server::TcpServer;
use samotop::smtp::SmtpParser;
use samotop::{io::smtp::SmtpService, mail::NullDispatch};
use std::sync::Arc;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::init();
    let mail_service = Builder::default()
        .using(NullDispatch)
        .using(Esmtp.with(SmtpParser))
        .into_service();
    let smtp_service = SmtpService::new(Arc::new(mail_service));

    TcpServer::on("localhost:2525").serve(smtp_service).await
}
