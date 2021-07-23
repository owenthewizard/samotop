/*!
Example of delivering to LMTP over unix socket (dovecot).
Maps recipients to local users per domain.

## Testing

```
sed -e 's/$/\r/' <<EOF | curl -v --url 'smtp://localhost:2525' \
--mail-from from@wow.example.com \
--mail-rcpt to@mikesh.example.com \
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
    mail::{Builder, Dir},
    smtp::{Esmtp, SmtpParser},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
    env_logger::init();
    task::block_on(main_fut())
}

#[cfg(not(unix))]
async fn main_fut() -> Result<()> {
    println!("This will only work on a unix-like system")
}

#[cfg(unix)]
async fn main_fut() -> Result<()> {
    use samotop::server::UnixServer;

    let service = Builder + Dir::new("tmp/samotop/spool/".into())? + Esmtp.with(SmtpParser);

    UnixServer::on("local.socket").serve(service.build()).await
}
