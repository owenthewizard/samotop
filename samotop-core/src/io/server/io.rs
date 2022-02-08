use super::{Server, Session};
use crate::{common::io, common::*, io::ConnectionInfo};
use async_std::stream::once;

pub struct StdIo;

impl Server for StdIo {
    fn sessions<'s, 'f>(
        &'s self,
    ) -> S1Fut<'f, Result<Pin<Box<dyn Stream<Item = Result<Session>> + Send + Sync>>>>
    where
        's: 'f,
    {
        let stream = RW {
            read: Box::pin(async_std::io::stdin()),
            write: Box::pin(async_std::io::stdout()),
        };
        let conn = ConnectionInfo::default();
        let mut session = Session::new(stream);
        session.store.set::<ConnectionInfo>(conn);

        let stream = Box::pin(ready(Ok(Box::pin(once(Ok(session)))
            as Pin<Box<dyn Stream<Item = Result<Session>> + Send + Sync>>)));

        stream
    }
}

struct RW<R, W> {
    read: Pin<Box<R>>,
    write: Pin<Box<W>>,
}

impl<R: io::Read, W> io::Read for RW<R, W> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        self.read.as_mut().poll_read(cx, buf)
    }
}

impl<R, W: io::Write> io::Write for RW<R, W> {
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
