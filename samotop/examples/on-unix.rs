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

#[cfg(all(unix, feature = "delivery"))]
#[async_std::main]
async fn main() -> Result<()> {
    use samotop::server::UnixServer;
    use samotop::{
        mail::{Builder, MailDir},
        smtp::{Esmtp, SmtpParser},
    };
    env_logger::init();

    let service = Builder + MailDir::new("tmp/samotop/spool/".into())? + Esmtp.with(SmtpParser);

    UnixServer::on("local.socket").serve(service.build()).await
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[cfg(not(all(unix, feature = "delivery")))]
#[async_std::main]
async fn main() -> Result<()> {
    panic!("This will only work on a unix-like system and with the delivery feature enabled.")
}
