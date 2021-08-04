/*!
Example of chastising bad/dummy clients for sending commands before the banner (spam/abuse control).

## Testing

```bash
cargo run --example prudent &
nc -C localhost 2525
```

If you issue a command before the timeout, a prudent error is shown.
If you wait long enough, the dummy service unavailable error is shown.

*/

use std::time::Duration;

use async_std::task;
use samotop::{
    mail::{Builder, Name},
    server::TcpServer,
    smtp::Prudence,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
    env_logger::init();
    task::block_on(main_fut())
}

async fn main_fut() -> Result<()> {
    let service = Builder
        + Name::new("prudent-dummy")
        + Prudence::default().with_banner_delay(Duration::from_millis(3210));

    TcpServer::on("localhost:2525").serve(service.build()).await
}
