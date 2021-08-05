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

#[cfg(any(feature = "parser-nom", feature = "parser-peg"))]
async fn main_fut() -> Result<()> {
    use samotop::mail::Builder;
    use samotop::mail::Name;
    use samotop::mail::NullDispatch;
    use samotop::mail::SessionLogger;
    use samotop::server::TcpServer;
    use samotop::smtp::{Esmtp, SmtpParser};

    let service = Builder
        + Name::new("samotop")
            .identify_instance(true)
            .identify_session(true)
        + SessionLogger
        + NullDispatch
        + Esmtp.with(SmtpParser);
    TcpServer::on("localhost:2525").serve(service.build()).await
}

#[cfg(not(any(feature = "parser-nom", feature = "parser-peg")))]
async fn main_fut() -> Result<()> {
    panic!("This will only work with some parser enabled - parser-peg or parser-nom")
}

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::init();
    main_fut().await
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
