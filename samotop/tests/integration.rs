use async_std::prelude::FutureExt;
use async_std::{
    channel::{Sender, TrySendError},
    io::{Cursor, Read},
};
use regex::Regex;
use samotop::{
    io::IoService,
    io::{tls::TlsCapable, ConnectionInfo},
    mail::{Builder, Name, NullDispatch},
    smtp::{Esmtp, Prudence, SmtpParser},
};
use samotop_core::common::*;
use std::time::Duration;

#[async_std::test]
async fn svc() -> Result<()> {
    let (s, r) = async_std::channel::unbounded();
    let read = Cursor::new(concat!(
        "ehlo macca\r\n",
        "mail from:<>\r\n",
        "rcpt to:<postmaster>\r\n",
        "data\r\n",
        "Subject: nice test\r\n",
        "\r\n",
        ".\r\n",
        "bugy command nonsense\r\n",
        "quit\r\n",
    ));
    let testio = TestIo::new(read, s);
    let io = Box::new(TlsCapable::plaintext(Box::new(testio)));
    let service = Builder + Name::new("testik") + NullDispatch + Esmtp.with(SmtpParser);

    service
        .build()
        .handle(Ok(io), ConnectionInfo::default())
        .await?;

    insta::assert_debug_snapshot!(
        String::from_utf8_lossy(r.recv().await?.as_slice()),
        @r###""220 testik service ready\r\n""###);
    insta::assert_debug_snapshot!(
        String::from_utf8_lossy(r.recv().await?.as_slice()),
        @r###""250 testik greets macca\r\n""###);
    insta::assert_debug_snapshot!(
        Regex::new("[0-9]{9}[0-9]*")?.replace(
        String::from_utf8_lossy(r.recv().await?.as_slice()).to_string().as_str(),"--redacted--"),
        @r###""250 Ok! Transaction --redacted-- started.\r\n""###);
    insta::assert_debug_snapshot!(
        String::from_utf8_lossy(r.recv().await?.as_slice()).to_string().as_str(),
        @r###""250 Ok\r\n""###);
    insta::assert_debug_snapshot!(
        String::from_utf8_lossy(r.recv().await?.as_slice()).to_string().as_str(),
        @r###""354 Start mail input, end with <CRLF>.<CRLF>\r\n""###);
    insta::assert_debug_snapshot!(
        Regex::new("[0-9]{9}[0-9]*")?.replace(
        String::from_utf8_lossy(r.recv().await?.as_slice()).to_string().as_str(),"--redacted--"),
        @r###""250 Queued as --redacted--\r\n""###);
    insta::assert_debug_snapshot!(
        String::from_utf8_lossy(r.recv().await?.as_slice()).to_string().as_str(),
        @r###""500 Syntax error, command unrecognized\r\n""###);
    insta::assert_debug_snapshot!(
        String::from_utf8_lossy(r.recv().await?.as_slice()),
        @r###""221 testik service closing transmission channel\r\n""###);

    assert!(r.recv().await.is_err(), "Should have no more");

    Ok(())
}

#[async_std::test]
async fn prudent_blocks_bad_client() -> Result<()> {
    let (s, r) = async_std::channel::unbounded();
    let read = Cursor::new(concat!("ehlo macca\r\n",));
    let testio = TestIo::new(read, s);
    let io = Box::new(TlsCapable::plaintext(Box::new(testio)));
    let service = Builder
        + Name::new("prudic")
        + Prudence::default().with_banner_delay(Duration::from_millis(50));

    service
        .build()
        .handle(Ok(io), ConnectionInfo::default())
        .await?;

    insta::assert_debug_snapshot!(
        String::from_utf8_lossy(r.recv().await?.as_slice()),
        @r###""451 Requested action aborted: error in processing.\r\n""###);

    assert!(r.recv().await.is_err(), "Should have no more");

    Ok(())
}

#[async_std::test]
async fn prudent_allows_good_client() -> Result<()> {
    let (s, r) = async_std::channel::unbounded();
    let read = DelayRead::new(100, Cursor::new(concat!("ehlo macca\r\n",)));
    let testio = TestIo::new(read, s);
    let io = Box::new(TlsCapable::plaintext(Box::new(testio)));
    let service = Builder
        + Name::new("prudic")
        + Esmtp.with(SmtpParser)
        + Prudence::default().with_banner_delay(Duration::from_millis(50));

    service
        .build()
        .handle(Ok(io), ConnectionInfo::default())
        .await?;

    insta::assert_debug_snapshot!(
        String::from_utf8_lossy(r.recv().await?.as_slice()),
        @r###""220 prudic service ready\r\n""###);
    insta::assert_debug_snapshot!(
        String::from_utf8_lossy(r.recv().await?.as_slice()),
        @r###""250 prudic greets macca\r\n""###);

    assert!(r.recv().await.is_err(), "Should have no more");

    Ok(())
}

struct DelayRead<R> {
    delay: Option<Pin<Box<dyn Future<Output = ()> + Sync + Send>>>,
    inner: R,
}

impl<R> DelayRead<R> {
    pub fn new(millis: u64, read: R) -> Self {
        Self {
            delay: Some(Box::pin(ready(()).delay(Duration::from_millis(millis)))),
            inner: read,
        }
    }
}

impl<R: Read + Unpin> Read for DelayRead<R> {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::result::Result<usize, std::io::Error>> {
        if let Some(ref mut delay) = self.delay {
            ready!(Pin::new(delay).poll(cx));
            self.delay = None;
        }
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
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

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

impl<R, W> TestIo<R, W> {
    pub fn new(read: R, write: W) -> Self {
        Self { read, write }
    }
}
