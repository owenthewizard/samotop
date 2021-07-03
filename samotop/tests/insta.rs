use async_std::{
    channel::{Sender, TrySendError},
    io::{Cursor, Read},
    prelude::FutureExt,
};
use regex::Regex;
use samotop::{
    io::IoService,
    io::{smtp::SmtpService, tls::TlsCapable, ConnectionInfo},
    mail::{Builder, Esmtp, NullDispatch},
    server::TcpServer,
    smtp::SmtpParser,
};
use samotop_core::common::*;
use std::{sync::Arc, time::Duration};

#[async_std::test]
async fn svc() -> Result<()> {
    let mail_service = Builder::default()
        .using(NullDispatch)
        .using(Esmtp.with(SmtpParser))
        .into_service();
    let smtp_service = SmtpService::new(Arc::new(mail_service));

    let (s, r) = async_std::channel::unbounded();
    let read = Cursor::new(concat!("ehlo macca\r\n", "mail from:<>\r\n", "quit\r\n"));
    let testio = TestIo::new(read, s);
    let io = Box::new(TlsCapable::plaintext(Box::new(testio)));

    smtp_service
        .handle(Ok(io), ConnectionInfo::default())
        .await?;

    TcpServer::on("localhost:2525")
        .serve(smtp_service)
        .timeout(Duration::from_secs(10))
        .await;

    insta::assert_debug_snapshot!(
        String::from_utf8_lossy(r.recv().await?.as_slice()),
        @r###""220 Service ready: samotop\r\n""###);
    insta::assert_debug_snapshot!(
        String::from_utf8_lossy(r.recv().await?.as_slice()),
        @r###""250 samotop greets macca\r\n""###);
    insta::assert_debug_snapshot!(
        Regex::new("[0-9]{9}[0-9]*")?.replace(
        String::from_utf8_lossy(r.recv().await?.as_slice()).to_string().as_str(),"--redacted--"),
        @r###""250 Ok! Transaction --redacted-- started.\r\n""###);
    insta::assert_debug_snapshot!(
        String::from_utf8_lossy(r.recv().await?.as_slice()),
        @r###""221 samotop Service closing transmission channel\r\n""###);

    assert!(r.recv().await.is_err(), "Should have no more");

    Ok(())
}

#[derive(Default, Debug)]
pub struct TestIo<R, W> {
    read: R,
    write: W,
}

impl<R: Read + Unpin, W: Unpin> Read for TestIo<R, W> {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        Pin::new(&mut self.read).poll_read(cx, buf)
    }
}

impl<R> Write for TestIo<R, Sender<Vec<u8>>> {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.write.try_send(buf.to_vec()) {
            Ok(()) => Poll::Ready(Ok(buf.len())),
            Err(TrySendError::Closed(_)) => {
                Poll::Ready(Err(io::Error::from(io::ErrorKind::NotConnected)))
            }
            Err(TrySendError::Full(_)) => Poll::Pending,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
// impl<R, W: Write + Unpin> Write for TestIo<R, W> {
//     fn poll_write(
//         mut self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//         buf: &[u8],
//     ) -> Poll<io::Result<usize>> {
//         Pin::new(&mut self.write).poll_write(cx, buf)
//     }

//     fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
//         Pin::new(&mut self.write).poll_flush(cx)
//     }

//     fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
//         Pin::new(&mut self.write).poll_close(cx)
//     }
// }

impl<R, W> TestIo<R, W> {
    pub fn new(read: R, write: W) -> Self {
        Self { read, write }
    }
}
