/*!
Example of delivering to LMTP over unix socket (dovecot).
Maps recipients to local users per domain.

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

#[cfg(feature = "delivery")]
#[async_std::main]
async fn main() -> Result<()> {
    use regex::Regex;
    use samotop::{
        io::client::{tls::NoTls, TcpConnector},
        mail::{Builder, LmtpDispatch, Mapper},
        server::TcpServer,
        smtp::{Esmtp, SmtpParser},
    };
    env_logger::init();

    let rcpt_map = Mapper::new(vec![
        (Regex::new(".*@(.*)")?, "$1@localhost".to_owned()), // use domain as a user name (all domain basket) anyone@example.org => example.org@localhost
        (Regex::new("[^@a-zA-Z0-9]+")?, "-".to_owned()), // sanitize the user name example.org@localhost => example-org@localhost
    ]);
    let lmtp_connector: TcpConnector<NoTls> = TcpConnector::default();
    let service = Builder
        + LmtpDispatch::new("dovecot:24".to_owned(), lmtp_connector)?.reuse(0)
        + rcpt_map
        + Esmtp.with(SmtpParser);

    TcpServer::on("localhost:2525").serve(service.build()).await
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[cfg(not(feature = "delivery"))]
#[async_std::main]
async fn main() -> Result<()> {
    panic!("This will only work with the delivery feature enabled.")
}
