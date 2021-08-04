/*!
Example of accepting an SMTP session on command IO
It stores the mail in a dir tmp/samotop/spool/

## Testing

```
sed -e 's/$/\r/' <<EOF | time cargo run --example on-cmd
lhlo boogie
mail from:<from@wow.example.com>
rcpt to:<komu@makesh.example.com>
rcpt to:<to@mikesh.example.com>
rcpt to:<za@mukesh.example.com>
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

#[cfg(feature = "delivery")]
#[async_std::main]
async fn main() -> Result<()> {
    use async_std::io::Read;
    use async_std::io::Write;
    use samotop::{
        io::{tls::TlsCapable, ConnectionInfo, IoService},
        mail::{Builder, MailDir},
        smtp::{Lmtp, SmtpParser},
    };
    use std::pin::Pin;
    use std::task::Context;
    use std::task::Poll;

    struct MyIo<R, W> {
        read: Pin<Box<R>>,
        write: Pin<Box<W>>,
    }

    impl<R: Read, W> Read for MyIo<R, W> {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<std::io::Result<usize>> {
            self.read.as_mut().poll_read(cx, buf)
        }
    }

    impl<R, W: Write> Write for MyIo<R, W> {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            self.write.as_mut().poll_write(cx, buf)
        }

        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            self.write.as_mut().poll_flush(cx)
        }

        fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            self.write.as_mut().poll_close(cx)
        }
    }

    env_logger::init();

    let service = Builder + MailDir::new("tmp/samotop/spool/".into())? + Lmtp.with(SmtpParser);

    let stream = MyIo {
        read: Box::pin(async_std::io::stdin()),
        write: Box::pin(async_std::io::stdout()),
    };
    let stream = TlsCapable::plaintext(Box::new(stream));
    let conn = ConnectionInfo::default();

    service.build().handle(Ok(Box::new(stream)), conn).await
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[cfg(not(feature = "delivery"))]
#[async_std::main]
async fn main() -> Result<()> {
    panic!("This will only work with the delivery feature enabled.")
}
