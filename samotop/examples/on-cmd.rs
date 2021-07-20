/*!
Example of accepting an SMTP session on command IO
It stores the mail in a dir tmp/samotop/spool/

## Testing

```
sed -e 's/$/\r/' <<EOF | time cargo run --example on-cmd
lhlo boogie
mail from:<from@spf.org>
rcpt to:<komu@makesh.info>
rcpt to:<to@mikesh.info>
rcpt to:<za@mukesh.info>
data
From: Moohoo <moo@hoo.com>
To: Yeeehaw <ye@haw.com>
Subject: Try me

xoxo
.
quit
EOF
find tmp/samotop/spool/new/ -print -exec cat {} \;
```
 */

use async_std::io::Read;
use async_std::io::Write;
use async_std::task;
use samotop::{
    io::{tls::TlsCapable, ConnectionInfo, IoService},
    mail::{Builder, Dir, Lmtp},
    smtp::SmtpParser,
};
use std::pin::Pin;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
    env_logger::init();
    task::block_on(main_fut())
}

async fn main_fut() -> Result<()> {
    let mail_service = Builder::default()
        .using(Dir::new("tmp/samotop/spool/".into())?)
        .using(Lmtp.with(SmtpParser))
        .build();

    let stream = MyIo {
        read: Box::pin(async_std::io::stdin()),
        write: Box::pin(async_std::io::stdout()),
    };
    let stream = TlsCapable::plaintext(Box::new(stream));
    let conn = ConnectionInfo::default();

    mail_service.handle(Ok(Box::new(stream)), conn).await
}

struct MyIo<R, W> {
    read: Pin<Box<R>>,
    write: Pin<Box<W>>,
}

impl<R: Read, W> Read for MyIo<R, W> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
        buf: &mut [u8],
    ) -> task::Poll<std::io::Result<usize>> {
        self.read.as_mut().poll_read(cx, buf)
    }
}

impl<R, W: Write> Write for MyIo<R, W> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
        buf: &[u8],
    ) -> task::Poll<std::io::Result<usize>> {
        self.write.as_mut().poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<std::io::Result<()>> {
        self.write.as_mut().poll_flush(cx)
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<std::io::Result<()>> {
        self.write.as_mut().poll_close(cx)
    }
}
