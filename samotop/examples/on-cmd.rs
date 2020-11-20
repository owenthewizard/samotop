/*!
Example of accepting an SMTP session on command IO
It stores the mail in a dir /tmp/samotop/spool/

## Testing

```
sed -e 's/$/\r/' <<EOF | cargo run --example on-cmd
lhlo boogie
mail from:<from@spf.org>
rcpt to:<to@mikesh.info>
data
From: Moohoo <moo@hoo.com>
To: Yeeehaw <ye@haw.com>
Subject: Try me

xoxo
.
quit
EOF
```
 */

use async_std::task;
use futures::AsyncRead as Read;
use futures::AsyncWrite as Write;
use samotop::{
    io::{smtp::SmtpService, tls::TlsEnabled, ConnectionInfo, IoService},
    mail::{Builder, Dir},
    parser::SmtpParser,
};
use std::pin::Pin;
use std::sync::Arc;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() -> Result<()> {
    env_logger::init();
    task::block_on(main_fut())
}

async fn main_fut() -> Result<()> {
    let dir_service = Dir::new("/tmp/samotop/spool/".into())?;
    let mail_service = Arc::new(Builder::default().using(dir_service));
    let smtp_service = SmtpService::new(Arc::new(mail_service), SmtpParser);
    let tls_smtp_service = TlsEnabled::disabled(smtp_service);

    let stream = MyIo {
        read: Box::pin(async_std::io::stdin()),
        write: Box::pin(async_std::io::stdout()),
    };
    let conn = ConnectionInfo::new(None, None);

    tls_smtp_service.handle(Ok(stream), conn).await
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
