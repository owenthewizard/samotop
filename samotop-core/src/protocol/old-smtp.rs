use crate::common::*;
use crate::io::tls::MayBeTls;
use crate::smtp::{ReadControl, WriteControl};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::{
    channel::mpsc::{Receiver, Sender},
    SinkExt,
};
use memchr::memchr;
use std::collections::VecDeque;

#[pin_project(project=SmtpCodecProj)]
pub struct SmtpCodec<IO> {
    /// the underlying IO, such as TcpStream
    #[pin]
    io: Option<IO>,
    /// server to client encoded responses buffer
    s2c_pending: VecDeque<WriteControl>,
    /// client to server reading buffer
    c2s_buffer: BytesMut,
    read_data: Option<bool>,
    sink: Sender<WriteControl>,
    recv: Receiver<WriteControl>,
}

impl<IO: Read + Write + MayBeTls> SmtpCodec<IO> {
    pub fn new(io: IO) -> Self {
        SmtpCodec::with_capacity(io, 1024)
    }
    pub fn with_capacity(io: IO, c2s_buffer_size: usize) -> Self {
        let (sink, recv) = futures::channel::mpsc::channel(1);
        SmtpCodec {
            io: Some(io),
            c2s_buffer: BytesMut::with_capacity(c2s_buffer_size),
            read_data: None,
            s2c_pending: vec![].into(),
            sink,
            recv,
        }
    }
    pub fn get_sender(&self) -> Sender<WriteControl> {
        self.sink.clone()
    }

    fn poll_read_buffer(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        let mut projection = self.project();
        let io = match projection.io.as_mut().as_pin_mut() {
            None => {
                trace!("Reading closed IO");
                return Poll::Ready(Ok(()));
            }
            Some(io) => io,
        };
        // fill the read buffer if all is read
        if projection.c2s_buffer.remaining() == 0 {
            // read more and decode new values
            trace!("Reading");
            if projection.c2s_buffer.remaining_mut() == 0 {
                projection.c2s_buffer.reserve(1024);
                trace!("Growing buffer to {}", projection.c2s_buffer.capacity());
            }
            let buf = projection.c2s_buffer.bytes_mut();
            // This is safe as long as poll_read fulfills the contract
            // TODO: What's the story with clippy::transmute_ptr_to_ptr?
            #[allow(clippy::transmute_ptr_to_ptr)]
            let buf = unsafe { std::mem::transmute(buf) };
            let len = ready!(io.poll_read(cx, buf))?;
            trace!("Read {} bytes.", len);
            // this is safe as long as poll_read fulfills the contract
            unsafe { projection.c2s_buffer.advance_mut(len) };
        }
        Poll::Ready(Ok(()))
    }

    fn read_line(self: Pin<&mut Self>) -> Option<Vec<u8>> {
        trace!("Reading next line");
        let projection = self.project();
        // process the read buffer into items
        let read = projection.c2s_buffer.bytes();
        if read.is_empty() {
            None
        } else {
            let read = match memchr(b'\n', read) {
                Some(len) => &read[..len + 1],
                None => read,
            };
            let bytes = Vec::from(read);
            projection.c2s_buffer.advance(bytes.len());
            Some(bytes)
        }
    }

    fn read_line_poll(self: Pin<&mut Self>) -> Poll<Option<Result<ReadControl>>> {
        Poll::Ready(self.read_line().map(|bytes| Ok(ReadControl::Raw(bytes))))
    }

    fn read_data_poll(self: Pin<&mut Self>) -> Poll<Option<Result<ReadControl>>> {
        let projection = self.project();
        trace!(
            "Reading next data from {} bytes",
            projection.c2s_buffer.remaining()
        );
        let nl = projection
            .read_data
            .expect("the caller should check for Some");
        if projection.c2s_buffer.remaining() == 0 {
            *projection.read_data = Some(nl);
            return Poll::Ready(None);
        }
        let consume = |buf: &mut BytesMut, len| {
            let bytes = Vec::from(&buf.bytes()[..len]);
            buf.advance(len);
            bytes
        };
        use DotState::*;
        match dotstate(&mut projection.c2s_buffer.iter(), nl) {
            Wait => {
                trace!("dotstate Wait");
                *projection.read_data = Some(nl);
                Poll::Pending
            }
            End(end) => {
                trace!("dotstate End {}", end);
                // it is the data terminating line
                *projection.read_data = None;
                let bytes = consume(projection.c2s_buffer, end);
                Poll::Ready(Some(Ok(ReadControl::EndOfMailData(bytes))))
            }
            EscapeDot => {
                trace!("dotstate EscapeDot");
                // the first byte is an escaping dot, send just the dot
                *projection.read_data = Some(false);
                let bytes = consume(projection.c2s_buffer, 1);
                Poll::Ready(Some(Ok(ReadControl::EscapeDot(bytes))))
            }
            CRLF => {
                trace!("dotstate CRLF");
                *projection.read_data = Some(true);
                let bytes = consume(projection.c2s_buffer, 2);
                Poll::Ready(Some(Ok(ReadControl::MailDataChunk(bytes))))
            }
            GoOn => match memchr(b'\r', projection.c2s_buffer.bytes()) {
                Some(found) => {
                    if let [b'\r', b'\n', ..] = projection.c2s_buffer[found..] {
                        *projection.read_data = Some(true);
                        let bytes = consume(projection.c2s_buffer, found + 2);
                        Poll::Ready(Some(Ok(ReadControl::MailDataChunk(bytes))))
                    } else {
                        *projection.read_data = Some(false);
                        let bytes = consume(projection.c2s_buffer, found);
                        Poll::Ready(Some(Ok(ReadControl::MailDataChunk(bytes))))
                    }
                }
                None => {
                    *projection.read_data = Some(false);
                    let bytes = consume(projection.c2s_buffer, projection.c2s_buffer.remaining());
                    Poll::Ready(Some(Ok(ReadControl::MailDataChunk(bytes))))
                }
            },
        }
    }

    fn poll_read_either(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<ReadControl>>> {
        // make sure any pending responses are written
        //ready!(self.as_mut().poll_flush(cx))?;
        // fill the buffer if necessary
        ready!(self.as_mut().poll_read_buffer(cx))?;

        if self.read_data.is_some() {
            self.read_data_poll()
        } else {
            self.read_line_poll()
        }
    }
    fn close_io(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        let mut projection = self.project();
        let io = match projection.io.as_mut().as_pin_mut() {
            None => {
                trace!("Closing closed IO");
                return Poll::Ready(Ok(()));
            }
            Some(io) => io,
        };
        ready!(io.poll_close(cx))?;
        projection.io.set(None);
        Poll::Ready(Ok(()))
    }
    fn write(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        let mut projection = self.as_mut().project();
        let mut io = match projection.io.as_mut().as_pin_mut() {
            None => return Poll::Ready(Err("Writing to a closed IO".into())),
            Some(io) => io,
        };
        loop {
            break match Pin::new(&mut projection.recv).poll_next(cx) {
                Poll::Ready(None) => Poll::Ready(Err("Other side shut down".into())),
                Poll::Ready(Some(c)) => {
                    trace!("Writing {:?}", c);
                    projection.s2c_pending.push_back(c);
                    continue;
                }
                Poll::Pending => {
                    if let Some(pending) = projection.s2c_pending.pop_front() {
                        // save bytes for next iteration
                        match pending {
                            WriteControl::Shutdown => {
                                trace!("shutting down");
                                ready!(self.close_io(cx))?;
                                Poll::Ready(Ok(()))
                            }
                            WriteControl::StartData => {
                                trace!("starting data");
                                *projection.read_data = Some(true);
                                continue;
                            }
                            WriteControl::StartTls => {
                                trace!("starting TLS");
                                io.as_mut().encrypt();
                                continue;
                            }
                            WriteControl::Data(mut pending) => {
                                // write data to the IO
                                trace!("writing {} bytes", pending.len());
                                match io.as_mut().poll_write(cx, &pending[..])? {
                                    Poll::Pending => {
                                        trace!("write not ready");
                                        // not ready, return the whole buffer to the queue
                                        projection
                                            .s2c_pending
                                            .push_front(PendingWrite::Data(pending));
                                        Poll::Pending
                                    }
                                    Poll::Ready(len) => {
                                        trace!("wrote {} bytes", len);
                                        let _consumed = pending.split_to(len);
                                        if !pending.is_empty() {
                                            // written partially, consume written buffer and return it to the queue
                                            projection
                                                .s2c_pending
                                                .push_front(PendingWrite::Data(pending));
                                        }
                                        continue;
                                    }
                                }
                            }
                        }
                    } else {
                        Poll::Ready(Ok(()))
                    }
                }
            };
        }
    }
    pub fn send(&self, response: WriteControl) -> S2Fut<Result<()>> {
        let mut sink = self.sink.clone();
        Box::pin(async move { Ok(sink.send(response).await?) })
    }
}

#[derive(Eq, PartialEq, Debug)]
pub enum DotState {
    /** Need more bytes */
    Wait,
    /** Data ending dot has been found (\r\n.\r\n => 5) */
    End(usize),
    /** Escaping dot has been found at position (\r\n..\r\n => 2) */
    EscapeDot,
    /** Carriage return and line feed were found and can be consumed => nl */
    CRLF,
    /** It's not a dot situation at all */
    GoOn,
}

pub fn dotstate<'a, I>(iter: &'a mut I, nl: bool) -> DotState
where
    I: Iterator<Item = &'a u8>,
{
    // Ok I see now why peeps complain about the bad design of SMTP!
    // This is a parser of the CR LF DOT CR LF situation.
    // Param 'nl' is flagged if the buffer comes as a new line.
    // That's important especially for the edge case of DATA\r\n.\r\n
    // (empty mail?) because the first set of CR LF is part of the command line.
    use self::DotState::*;
    match iter.next() {
        None => Wait,
        Some(b0) => match (nl, b0) {
            (true, b'.') => match iter.next() {
                None => Wait,
                Some(b'\r') => match iter.next() {
                    None => Wait,
                    Some(b'\n') => End(3),
                    Some(_) => EscapeDot,
                },
                Some(_) => EscapeDot,
            },
            (true, b'\r') => match iter.next() {
                None => Wait,
                Some(b'\n') => CRLF,
                Some(_) => GoOn,
            },
            (true, _) => GoOn,
            (false, b'\r') => match iter.next() {
                None => Wait,
                Some(b'\n') => CRLF,
                Some(_) => GoOn,
            },
            (false, _) => GoOn,
        },
    }
}

impl<IO> Stream for SmtpCodec<IO>
where
    IO: Read + Write + MayBeTls,
{
    type Item = Result<ReadControl>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        ready!(self.as_mut().write(cx))?;

        match self.as_mut().poll_read_either(cx) {
            Poll::Ready(None) => {
                ready!(self.close_io(cx))?;
                Poll::Ready(None)
            }
            Poll::Ready(Some(ready)) => Poll::Ready(Some(ready)),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod dotstate_tests {
    use super::dotstate;
    use super::DotState::*;

    #[test]
    fn dotstate_handles_empty_line() {
        let r = dotstate(&mut b".\r\n".iter(), true);
        assert_eq!(r, End(3));
        let r = dotstate(&mut b"\r\n.\r\n".iter(), false);
        assert_eq!(r, CRLF);
    }

    #[test]
    fn dotstate_ignores_lf_only() {
        let r = dotstate(&mut b".\n".iter(), false);
        assert_eq!(r, GoOn);
        let r = dotstate(&mut b"\n.\n".iter(), false);
        assert_eq!(r, GoOn);
    }

    #[test]
    fn dotstate_handles_escape_dot() {
        let r = dotstate(&mut b"..\r\n".iter(), true);
        assert_eq!(r, EscapeDot);
        let r = dotstate(&mut b"..xxx\r\n".iter(), true);
        assert_eq!(r, EscapeDot);
        let r = dotstate(&mut b".xxx\r\n".iter(), true);
        assert_eq!(r, EscapeDot);
        let r = dotstate(&mut b"\r\n..\r\n".iter(), false);
        assert_eq!(r, CRLF);
        let r = dotstate(&mut b"\r\n.xxx\r\n".iter(), false);
        assert_eq!(r, CRLF);
        let r = dotstate(&mut b"\n..\n".iter(), false);
        assert_eq!(r, GoOn);
        let r = dotstate(&mut b"\n.xxx\n".iter(), false);
        assert_eq!(r, GoOn);
    }

    #[test]
    fn dotstate_handles_missing_bytes() {
        let r = dotstate(&mut b".".iter(), true);
        assert_eq!(r, Wait);
        let r = dotstate(&mut b".\r".iter(), true);
        assert_eq!(r, Wait);
        let r = dotstate(&mut b"\r".iter(), false);
        assert_eq!(r, Wait);
        let r = dotstate(&mut b"\r\n".iter(), false);
        assert_eq!(r, CRLF);
        let r = dotstate(&mut b"\r\n.".iter(), false);
        assert_eq!(r, CRLF);
        let r = dotstate(&mut b"\r\n.\r".iter(), false);
        assert_eq!(r, CRLF);
    }
}

/*

helo there
mail from:<gorila@mozilla.ff>
rcpt to:<stalin@hell.hot>
data
BOOOO
.
mail from:<banana@mozilla.ff>
rcpt to:<hitler@hell.hot>
data
BAAAA
.
mail from:<ticktack@mozilla.ff>
rcpt to:<trump@hell.hot>
data
DRRRR
.
QUIT

*/

#[cfg(test)]
mod codec_tests {
    use super::*;
    use crate::smtp::SmtpReply;
    use crate::test_util::*;
    use ReadControl::*;

    #[test]
    fn decode_takes_first_line() -> Result<()> {
        let mut io = TestIO::default()
            .add_read_chunk("helo there\r\n")
            .add_read_chunk("quit\r\n");
        let mut sut = SmtpCodec::new(&mut io);

        let res = Pin::new(&mut sut).poll_next(&mut cx())?;
        if let Poll::Ready(Some(Raw(bytes))) = res {
            assert_eq!(bytes, b("helo there\r\n"));
        } else {
            panic!("invalid result. Expected Raw, got {:?}", res);
        }
        assert_eq!(b(io.read()), b("helo there\r\n"));
        Ok(())
    }

    #[test]
    fn decode_returns_any_command_line() -> Result<()> {
        let io = TestIO::from(b"he\r\n".to_vec());
        let mut sut = SmtpCodec::new(io);

        match Pin::new(&mut sut).poll_next(&mut cx())? {
            Poll::Ready(Some(Raw(bytes))) => assert_eq!(bytes, b("he\r\n")),
            res => panic!("invalid result. Expected Raw, got {:?}", res),
        }
        Ok(())
    }

    #[test]
    fn decode_handles_weird_command() -> Result<()> {
        let io = TestIO::from(b"!@#\r\nquit\r\n".to_vec());
        let mut sut = SmtpCodec::new(io);

        match Pin::new(&mut sut).poll_next(&mut cx())? {
            Poll::Ready(Some(Raw(bytes))) => assert_eq!(bytes, b("!@#\r\n")),
            res => panic!("invalid result. Expected Raw, got {:?}", res),
        }
        match Pin::new(&mut sut).poll_next(&mut cx())? {
            Poll::Ready(Some(Raw(bytes))) => assert_eq!(bytes, b("quit\r\n")),
            res => panic!("invalid result. Expected Raw, got {:?}", res),
        }
        Ok(())
    }

    #[test]
    fn decode_handles_empty_data_buffer() -> Result<()> {
        let io = TestIO::from(b"data\r\n".to_vec());
        let mut sut = SmtpCodec::new(io);

        match Pin::new(&mut sut).poll_next(&mut cx())? {
            Poll::Ready(Some(Raw(bytes))) => assert_eq!(bytes, b("data\r\n")),
            res => panic!("invalid result. Expected Raw, got {:?}", res),
        }
        match Pin::new(&mut sut).poll_next(&mut cx())? {
            Poll::Ready(None) => (/*OK*/),
            res => panic!("invalid result. Expected None, got {:?}", res),
        }
        Ok(())
    }

    #[test]
    fn decode_finds_data_dot() -> Result<()> {
        let io = TestIO::from(b"something\r\n..fun\r\n.\r\nCOMMAND\r\n".to_vec());
        let mut sut = SmtpCodec::new(io);
        let mut sender = sut.get_sender();

        match Pin::new(&mut sut).poll_next(&mut cx())? {
            Poll::Ready(Some(Raw(bytes))) => assert_eq!(bytes, b("something\r\n")),
            res => panic!("invalid result. Expected Raw, got {:?}", res),
        }

        assert_eq!(sender.poll_ready(&mut cx())?, Poll::Ready(()));
        assert_eq!(
            sender.start_send(WriteControl::StartData(SmtpReply::StartMailInputChallenge))?,
            ()
        );

        match Pin::new(&mut sut).poll_next(&mut cx())? {
            Poll::Ready(Some(EscapeDot(bytes))) => assert_eq!(bytes, b(".")),
            res => panic!("invalid result. Expected escape dot, got {:?}", res),
        }
        match Pin::new(&mut sut).poll_next(&mut cx())? {
            Poll::Ready(Some(MailDataChunk(bytes))) => assert_eq!(bytes, b(".fun\r\n")),
            res => panic!("invalid result. Expected data chunk, got {:?}", res),
        }
        match Pin::new(&mut sut).poll_next(&mut cx())? {
            Poll::Ready(Some(EndOfMailData(bytes))) => assert_eq!(bytes, b(b".\r\n")),
            res => panic!("invalid result. Expected data end, got {:?}", res),
        }
        match Pin::new(&mut sut).poll_next(&mut cx())? {
            Poll::Ready(Some(Raw(bytes))) => assert_eq!(bytes, b(b"COMMAND\r\n")),
            res => panic!("invalid result. Expected command, got {:?}", res),
        }
        match Pin::new(&mut sut).poll_next(&mut cx())? {
            Poll::Ready(None) => (/*OK*/),
            res => panic!("invalid result. Expected None, got {:?}", res),
        }
        Ok(())
    }

    #[test]
    fn decode_finds_data_dot_after_empty_data() -> Result<()> {
        let io = TestIO::from(b".\r\n".to_vec());
        let mut sut = SmtpCodec::new(io);
        let mut sender = sut.get_sender();

        assert_eq!(sender.poll_ready(&mut cx())?, Poll::Ready(()));
        assert_eq!(
            sender.start_send(WriteControl::StartData(SmtpReply::StartMailInputChallenge))?,
            ()
        );
        match Pin::new(&mut sut).poll_next(&mut cx())? {
            Poll::Ready(Some(EndOfMailData(bytes))) => assert_eq!(bytes, b(b".\r\n")),
            res => panic!("invalid result. Expected data end, got {:?}", res),
        }
        Ok(())
    }
}
