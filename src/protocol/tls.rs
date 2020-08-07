use crate::common::*;
use crate::model::io::*;
use async_tls::server::TlsStream;
use async_tls::Accept;
use async_tls::TlsAcceptor;
use futures::channel::oneshot::{channel, Canceled, Receiver, Sender};
use pin_project::pin_project;

pub trait IntoTlsUpgrade
where
    Self: Sized,
{
    fn tls_upgrade(self, switch: TlsSwitch) -> TlsUpgrade<Self> {
        TlsUpgrade::new(self, switch)
    }
}

impl<T> IntoTlsUpgrade for T where T: Stream<Item = Result<WriteControl>> {}

#[pin_project(project=TlsUpgradeProjection)]
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct TlsUpgrade<S> {
    #[pin]
    stream: S,
    switch: TlsSwitch,
}

impl<S> TlsUpgrade<S> {
    pub fn new(stream: S, switch: TlsSwitch) -> Self {
        Self { stream, switch }
    }
}

impl<S> Stream for TlsUpgrade<S>
where
    S: Stream<Item = Result<WriteControl>>,
{
    type Item = S::Item;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let TlsUpgradeProjection { stream, switch } = self.project();
        match ready!(stream.poll_next(cx)) {
            starttls @ Some(Ok(WriteControl::StartTls)) => {
                switch.start_tls()?;
                Poll::Ready(starttls)
            }
            pass => Poll::Ready(pass),
        }
    }
}

impl<S> WillDoTls for S where S: Read + Write + Unpin {}

pub trait WillDoTls
where
    Self: Sized,
{
    fn with_tls(
        self,
        acceptor: Option<TlsAcceptor>,
        mode: TlsMode,
    ) -> (TlsSwitch, TlsCapable<Self>) {
        tls_capable(self, acceptor, mode)
    }
}

pub fn tls_capable<IO>(
    io: IO,
    acceptor: Option<TlsAcceptor>,
    mut mode: TlsMode,
) -> (TlsSwitch, TlsCapable<IO>) {
    let (sender, receiver) = channel();
    if acceptor.is_none() {
        mode = TlsMode::Disabled;
    }
    let switch = match mode {
        TlsMode::Disabled => TlsSwitch::Canceled,
        TlsMode::Enabled => TlsSwitch::Sent,
        _ => TlsSwitch::new(sender),
    };
    let starter = TlsStarter::new(receiver, mode);
    let io = match mode {
        TlsMode::Disabled => Tls::PlainText(io),
        TlsMode::Enabled => Tls::Enabled(io, acceptor.expect("taken care of earlier"), starter),
        TlsMode::StartTls => Tls::Enabled(io, acceptor.expect("taken care of earlier"), starter),
    };
    (switch, io)
}

#[derive(Debug)]
pub enum TlsSwitch {
    Pending(Sender<()>),
    Canceled,
    Sent,
}

impl TlsSwitch {
    pub fn new(sender: Sender<()>) -> Self {
        TlsSwitch::Pending(sender)
    }
    pub fn start_tls(&mut self) -> Result<()> {
        *self = match std::mem::replace(self, TlsSwitch::Canceled) {
            TlsSwitch::Pending(sender) => {
                sender
                    .send(())
                    .map_err(|e| format!("Failed to send start_tls signal: {:?}", e))?;
                TlsSwitch::Sent
            }
            otherwise => otherwise,
        };
        Ok(())
    }
}

pub struct TlsStarter {
    signal: Receiver<()>,
    mode: TlsMode,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TlsMode {
    Disabled,
    Enabled,
    StartTls,
}

impl TlsStarter {
    pub fn new(signal: Receiver<()>, mode: TlsMode) -> Self {
        Self { signal, mode }
    }
    fn check_tls(&mut self) -> TlsMode {
        let mode = match self.mode {
            TlsMode::Disabled => TlsMode::Disabled,
            TlsMode::Enabled => TlsMode::Enabled,
            m => match self.signal.try_recv() {
                Ok(Some(())) => TlsMode::Enabled,
                Ok(None) => m,
                Err(Canceled) => TlsMode::Disabled,
            },
        };
        trace!("TLS is: {:?}, was {:?}", mode, self.mode);
        self.mode = mode;
        self.mode
    }
    pub fn should_start_tls(&mut self) -> bool {
        match self.check_tls() {
            TlsMode::Enabled => true,
            _ => false,
        }
    }
}

pub enum TlsCapable<IO> {
    PlainText(IO),
    Enabled(IO, TlsAcceptor, TlsStarter),
    HandShake(Accept<IO>),
    Encrypted(TlsStream<IO>),
    Failed,
}
type Tls<T> = TlsCapable<T>;

impl<IO: Read + Write + Unpin> Tls<IO> {
    fn poll_tls(&mut self, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match std::mem::replace(self, Tls::Failed) {
            Tls::Enabled(io, acceptor, mut starter) => {
                if starter.should_start_tls() {
                    trace!("Switching to TLS");
                    // Calling `acceptor.accept` will start the TLS handshake
                    let handshake = acceptor.clone().accept(io);
                    // The handshake is a future we can await to get an encrypted
                    // stream back.
                    *self = Tls::HandShake(handshake);
                    self.poll_tls(cx)
                } else {
                    *self = Tls::Enabled(io, acceptor, starter);
                    Poll::Ready(Ok(()))
                }
            }
            Tls::HandShake(mut handshake) => {
                trace!("Waiting for TLS handshake");
                match Pin::new(&mut handshake).poll(cx) {
                    Poll::Ready(Err(e)) => {
                        *self = Tls::Failed;
                        Poll::Ready(Err(e.into()))
                    }
                    Poll::Pending => {
                        trace!("TLS is not ready");
                        *self = Tls::HandShake(handshake);
                        Poll::Pending
                    }
                    Poll::Ready(Ok(stream)) => {
                        trace!("TLS is on!");
                        *self = Tls::Encrypted(stream);
                        Poll::Ready(Ok(()))
                    }
                }
            }
            current @ Tls::PlainText(_) => {
                *self = current;
                Poll::Ready(Ok(()))
            }
            current @ Tls::Encrypted(_) => {
                *self = current;
                Poll::Ready(Ok(()))
            }
            current @ Tls::Failed => {
                *self = current;
                Self::ready_failed()
            }
        }
    }
    fn failed() -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Tls setup failed")
    }
    fn ready_failed<T>() -> Poll<std::io::Result<T>> {
        Poll::Ready(Err(Self::failed()))
    }
}
impl<IO> std::fmt::Debug for Tls<IO> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        match self {
            Tls::PlainText(_) => write!(f, "PlainText(...)"),
            Tls::Enabled(_, _, _) => write!(f, "Enabled(...)"),
            Tls::HandShake(_) => write!(f, "HandShake(...)"),
            Tls::Encrypted(_) => write!(f, "Encrypted(...)"),
            Tls::Failed => write!(f, "Failed"),
        }
    }
}

impl<IO: Read + Write + Unpin> Read for Tls<IO> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::result::Result<usize, std::io::Error>> {
        ready!(self.poll_tls(cx))?;
        match &mut *self {
            Tls::Encrypted(ref mut io) => Pin::new(io).poll_read(cx, buf),
            Tls::PlainText(ref mut io) => Pin::new(io).poll_read(cx, buf),
            Tls::Enabled(ref mut io, _, _) => Pin::new(io).poll_read(cx, buf),
            Tls::HandShake(_) => unreachable!("this path is handled in poll_tls()"),
            Tls::Failed => Self::ready_failed(),
        }
    }
}

impl<IO: Read + Write + Unpin> Write for Tls<IO> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match &mut *self {
            Tls::Encrypted(ref mut io) => Pin::new(io).poll_write(cx, buf),
            Tls::PlainText(ref mut io) => Pin::new(io).poll_write(cx, buf),
            Tls::Enabled(ref mut io, _, _) => Pin::new(io).poll_write(cx, buf),
            Tls::HandShake(_) => todo!("What to do here. Polling TLS on write deadlocks"),
            Tls::Failed => Self::ready_failed(),
        }
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        ready!(self.poll_tls(cx))?;
        match &mut *self {
            Tls::Encrypted(ref mut io) => Pin::new(io).poll_flush(cx),
            Tls::PlainText(ref mut io) => Pin::new(io).poll_flush(cx),
            Tls::Enabled(ref mut io, _, _) => Pin::new(io).poll_flush(cx),
            Tls::HandShake(_) => unreachable!("this path is handled in poll_tls()"),
            Tls::Failed => Self::ready_failed(),
        }
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        ready!(self.poll_tls(cx))?;
        match &mut *self {
            Tls::Encrypted(ref mut io) => Pin::new(io).poll_close(cx),
            Tls::PlainText(ref mut io) => Pin::new(io).poll_close(cx),
            Tls::Enabled(ref mut io, _, _) => Pin::new(io).poll_close(cx),
            Tls::HandShake(_) => unreachable!("this path is handled in poll_tls()"),
            Tls::Failed => Self::ready_failed(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Result;
    use async_std::io::Cursor;
    use futures_await_test::async_test;
    use pin_project::pin_project;
    use rustls::ServerConfig;
    use std::pin::Pin;

    #[pin_project]
    struct TestIO<R: Read, W: Write> {
        #[pin]
        pub read: R,
        #[pin]
        pub write: W,
    }

    impl<R: Read, W: Write> Read for TestIO<R, W> {
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &mut [u8],
        ) -> Poll<std::io::Result<usize>> {
            self.project().read.poll_read(cx, buf)
        }
    }

    impl<R: Read, W: Write> Write for TestIO<R, W> {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            self.project().write.poll_write(cx, buf)
        }
        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<std::io::Result<()>> {
            self.project().write.poll_flush(cx)
        }
        fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<std::io::Result<()>> {
            self.project().write.poll_close(cx)
        }
    }

    #[async_test]
    async fn test_starttls_plaintext() -> Result<()> {
        let inp = b"STARTTLS\r\n";
        let mut outp = [0u8; 4096];
        let mut io = TestIO {
            read: Cursor::new(&inp[..]),
            write: Cursor::new(&mut outp[..]),
        };

        let (_, mut tls) = tls_capable(&mut io, None, TlsMode::Disabled);

        tls.write(b"220 OK\r\n").await?;

        assert_eq!(8, io.write.position());

        Ok(())
    }
}
