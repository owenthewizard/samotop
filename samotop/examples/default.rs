/*!
Example of delivering nowhere except to the console output

## Testing

```
curl -v --url 'smtp://localhost:2525' \
--mail-from from@wow.example.com \
--mail-rcpt to@mikesh.example.com \
--upload-file - <<EOF
From: Moohoo <moo@hoo.com>
To: Yeeehaw <ye@haw.com>
Subject: Try me

xoxo
EOF
```

 */

use samotop::mail::Builder;
use samotop::mail::NullDispatch;
use samotop::server::TcpServer;
use samotop::smtp::{Esmtp, SmtpParser};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::init();
    let service = Builder + NullDispatch + Esmtp.with(SmtpParser);

    TcpServer::on("localhost:2525").serve(service.build()).await
}
