/*!
Example of delivering to LMTP over a child process IO.
Maps recipients to local users per domain.

## Testing

```
nc -C localhost 2525 <<EOF
lhlo mememe
mail from:<from@wow.example.com>
rcpt to:<to@mikesh.example.com>
rcpt to:<tree@mikesh.example.com>
rcpt to:<flour@mikesh.example.com>
data
From: Moohoo <moo@hoo.com>
To: Yeeehaw <ye@haw.com>
Subject: Try me

xoxo
.
EOF
```
 */

use async_std::task;
use regex::Regex;
use samotop::{
    io::client::tls::NoTls,
    mail::{Builder, Lmtp, LmtpDispatch, Mapper},
    server::TcpServer,
    smtp::SmtpParser,
};

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
    use samotop::io::client::ChildConnector;
    let lmtp_connector: ChildConnector<NoTls> = ChildConnector::default();
    let mail_service = Builder::default()
        .using(
            LmtpDispatch::new(
                "samotop/examples/to-lmtp-child.sh".to_owned(),
                lmtp_connector,
            )?
            .reuse(0),
        )
        .using(rcpt_map)
        .using(Lmtp.with(SmtpParser))
        .build();

    TcpServer::on("localhost:2525").serve(mail_service).await
}
