/*!
Example of delivering to LMTP over unix socket

## Testing

```
curl --url 'smtp://localhost:2525' \
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
use samotop::server::Server;
use samotop::service::mail::default::DefaultMailService;
use samotop::service::parser::SmtpParser;
use samotop::service::tcp::{smtp::SmtpService, tls::TlsEnabled};
use std::sync::Arc;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
    env_logger::init();
    task::block_on(main_fut())
}

async fn main_fut() -> Result<()> {
    let mail_service = DefaultMailService::new("test-samotop".to_owned());
    let smtp_service = SmtpService::new(Arc::new(mail_service), SmtpParser);
    let tls_smtp_service = TlsEnabled::disabled(smtp_service);
    Server::on("localhost:2525").serve(tls_smtp_service).await
}
