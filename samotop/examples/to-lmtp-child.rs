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

#[cfg(all(feature = "delivery", feature = "mapper"))]
#[async_std::main]
async fn main() -> Result<()> {
    use regex::Regex;
    use samotop::{
        io::client::tls::NoTls,
        mail::{Builder, LmtpDispatch, Mapper},
        server::TcpServer,
        smtp::{Lmtp, SmtpParser},
    };

    env_logger::init();

    let rcpt_map = Mapper::new(vec![
        (Regex::new(".*@(.*)")?, "$1@localhost".to_owned()), // use domain as a user name (all domain basket) anyone@example.org => example.org@localhost
        (Regex::new("[^@a-zA-Z0-9]+")?, "-".to_owned()), // sanitize the user name example.org@localhost => example-org@localhost
    ]);
    use samotop::io::client::ChildConnector;
    let lmtp_connector: ChildConnector<NoTls> = ChildConnector::default();
    let service = Builder
        + LmtpDispatch::new(
            "samotop/examples/to-lmtp-child.sh".to_owned(),
            lmtp_connector,
        )?
        .reuse(0)
        + rcpt_map
        + Lmtp.with(SmtpParser);

    TcpServer::on("localhost:2525").serve(service.build()).await
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[cfg(not(all(feature = "delivery", feature = "mapper")))]
#[async_std::main]
async fn main() -> Result<()> {
    panic!("This will only work with the delivery feature enabled.")
}
