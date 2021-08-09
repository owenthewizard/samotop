/*!
Example of receiving IMAP commands

## Testing - https://gist.github.com/akpoff/53ac391037ae2f2d376214eac4a23634

 */

use async_std::{
    io::{
        self,
        prelude::{BufRead, WriteExt},
        stdout, BufReader, Read, Write,
    },
    task::ready,
};
use log::*;
use samotop::{io::IoService, server::TcpServer};
use std::task::{Context, Poll};
use std::{fmt, future::Future};
use std::{ops::RangeBounds, pin::Pin};

#[async_std::main]
async fn main() -> Result<()> {
    env_logger::init();
    TcpServer::on("localhost:2525").serve(Imap).await
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

struct Imap;

impl IoService for Imap {
    fn handle(
        &self,
        io: samotop_core::common::Result<Box<dyn samotop::io::tls::MayBeTls>>,
        _connection: samotop::io::ConnectionInfo,
    ) -> samotop_core::common::S1Fut<'static, samotop_core::common::Result<()>> {
        Box::pin(async move {
            let mut io = BufReader::new(io?);
            let io = &mut io;

            respond(io.get_mut(), "*", "OK Server ready.").await?;

            #[derive(Debug, Default)]
            struct State {
                usr: String,
                pwd: String,
            }
            let mut state = State::default();

            loop {
                let tag = not(ws()).parse_str(io, 1..).await?;
                ws().skip(io, 1..).await?;
                let cmd = not(ws()).parse_str(io, 1..).await?;

                info!("{} {}", tag, cmd);

                match cmd.to_ascii_lowercase().as_str() {
                    "capability" => {
                        respond(io.get_mut(), tag, "OK NONE").await?;
                    }
                    "login" => {
                        ws().skip(io, 1..).await?;
                        state.usr = not(ws()).parse_str(io, 1..).await?;
                        ws().skip(io, 1..).await?;
                        state.pwd = not(ws()).parse_str(io, 1..).await?;
                        info!("USER {} PWD {}", state.usr, state.pwd.len());
                        respond(io.get_mut(), tag, "OK authhhh").await?;
                    }
                    "status" => {
                        ws().skip(io, 1..).await?;
                        let inbx = not(ws()).parse_str(io, 1..).await?;
                        respond(io.get_mut(), "*", format!("STATUS {} (MESSAGES 231)", inbx))
                            .await?;
                        respond(io.get_mut(), tag, "OK STATUS completed").await?;
                    }
                    "select" => {
                        ws().skip(io, 1..).await?;
                        let inbx = not(ws()).parse_str(io, 1..).await?;
                        respond(io.get_mut(), "*", "172 EXISTS").await?;
                        respond(io.get_mut(), "*", "1 RECENT").await?;
                        respond(
                            io.get_mut(),
                            "*",
                            "OK [UNSEEN 12] Message 12 is first unseen",
                        )
                        .await?;
                        respond(io.get_mut(), "*", "OK [UIDVALIDITY 3857529045] UIDs valid")
                            .await?;
                        respond(io.get_mut(), "*", "OK [UIDNEXT 4392] Predicted next UID").await?;
                        respond(
                            io.get_mut(),
                            "*",
                            "FLAGS (\\Answered \\Flagged \\Deleted \\Seen \\Draft)",
                        )
                        .await?;
                        respond(
                            io.get_mut(),
                            "*",
                            "OK [PERMANENTFLAGS (\\Deleted \\Seen \\*)] Limited",
                        )
                        .await?;
                        respond(io.get_mut(), tag, "OK SELECT [READ-ONLY] completed").await?;
                    }
                    "logout" => {
                        respond(io.get_mut(), tag, "OK bye").await?;
                        break;
                    }
                    _ => {
                        respond(io.get_mut(), tag, "NO unimplemented!").await?;
                    }
                }
                not(eol()).skip(io, ..).await?;
                eol().skip(io, 1..).await?;
            }
            async_std::io::copy(io, stdout()).await?;
            Ok(())
        })
    }
}

type Fut<'f, T> = Pin<Box<dyn Future<Output = T> + Send + 'f>>;

pub trait Parse {
    fn parse<'a, 'i, 'r, IO, R>(
        &'a mut self,
        io: &'i mut BufReader<IO>,
        range: R,
    ) -> Fut<'i, io::Result<Vec<u8>>>
    where
        'a: 'i,
        'r: 'i,
        IO: Read + Send + Unpin,
        R: 'r + RangeBounds<u8> + Send + Unpin,
        Self: Send;
    fn parse_str<'a, 'i, 'r, IO, R>(
        &'a mut self,
        io: &'i mut BufReader<IO>,
        range: R,
    ) -> Fut<'i, io::Result<String>>
    where
        'a: 'i,
        'r: 'i,
        IO: Read + Send + Unpin,
        R: 'r + RangeBounds<u8> + Send + Unpin,
        Self: Send,
    {
        Box::pin(async move {
            let bytes = self.parse(io, range).await?;
            let result = match String::from_utf8(bytes) {
                Err(_) => return Err(io::ErrorKind::InvalidInput.into()),
                Ok(v) => v,
            };
            debug!("Incoming string: {:?}", result);
            Ok(result)
        })
    }

    fn skip<'a, 'i, 'r, IO, R>(
        &'a mut self,
        io: &'i mut BufReader<IO>,
        range: R,
    ) -> Fut<'i, io::Result<usize>>
    where
        'a: 'i,
        'r: 'i,
        IO: Read + Unpin + Send,
        Self: Send,
        R: 'r + RangeBounds<u8> + Send + Unpin,
    {
        Box::pin(async move {
            let bytes = self.parse(io, range).await?;
            debug!("Skipping: {:?}", bytes.len());
            Ok(bytes.len())
        })
    }
}
impl<T> Parse for T
where
    T: FnMut(&u8) -> bool,
{
    fn parse<'a, 'i, 'r, IO, R>(
        &'a mut self,
        io: &'i mut BufReader<IO>,
        range: R,
    ) -> Fut<'i, io::Result<Vec<u8>>>
    where
        'a: 'i,
        'r: 'i,
        IO: Read + Send + Unpin,
        R: 'r + RangeBounds<u8> + Send + Unpin,
        Self: Send,
    {
        Box::pin(async move { parse(io, range, self).await })
    }
}

async fn respond<IO: Write + Unpin>(
    mut io: IO,
    tag: impl fmt::Display,
    what: impl fmt::Display,
) -> io::Result<()> {
    let msg = format!("{} {}\r\n", tag, what);
    io.write_all(msg.as_bytes()).await?;
    io.flush().await?;
    debug!("Response: {:?}", msg);
    Ok(())
}

async fn parse<'a, 'i, IO: Read + Unpin, P: 'a + FnMut(&u8) -> bool, R: RangeBounds<u8>>(
    io: &'i mut BufReader<IO>,
    range: R,
    mut predicate: P,
) -> io::Result<Vec<u8>> {
    struct Fill<'a, T>(&'a mut BufReader<T>);
    impl<'a, T: Read + Unpin> Future for Fill<'a, T> {
        type Output = io::Result<()>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            ready!(Pin::new(&mut self.get_mut().0).poll_fill_buf(cx))?;
            Poll::Ready(Ok(()))
        }
    }

    let min = match range.start_bound() {
        std::ops::Bound::Unbounded => 0,
        std::ops::Bound::Included(min) => *min,
        std::ops::Bound::Excluded(min) => min.saturating_add(1),
    } as usize;
    let max = match range.end_bound() {
        std::ops::Bound::Unbounded => u8::MAX,
        std::ops::Bound::Included(max) => *max,
        std::ops::Bound::Excluded(max) => max.saturating_sub(1),
    } as usize;

    let mut result = vec![];
    loop {
        if io.buffer().is_empty() {
            Fill(io).await?;
        }
        let buf = io.buffer();
        let taken = buf
            .iter()
            .take(max.saturating_sub(result.len()))
            .take_while(|b| predicate(*b))
            .count();
        let stop = taken < buf.len();
        if taken != 0 {
            result.extend_from_slice(&buf[0..taken]);
            Pin::new(&mut *io).consume(taken);
        }
        if stop {
            break;
        }
    }

    debug!(
        "Incoming bytes: {:?}",
        String::from_utf8_lossy(result.as_ref())
    );

    if result.len() < min {
        Err(io::ErrorKind::InvalidInput.into())
    } else {
        Ok(result)
    }
}

pub fn not(mut predicate: impl FnMut(&u8) -> bool) -> impl FnMut(&u8) -> bool {
    move |x| !predicate(x)
}
pub fn ws() -> impl FnMut(&u8) -> bool {
    u8::is_ascii_whitespace
}
pub fn eol() -> impl FnMut(&u8) -> bool {
    |b| b == &b'\n' || b == &b'\r'
}

pub trait Check {
    fn parse<'a, 'i, IO>(&'a mut self, io: &'i mut BufReader<IO>) -> Fut<'i, io::Result<Vec<u8>>>
    where
        'a: 'i,
        IO: Read + Send + Unpin,
        Self: Send;
}

impl<T> Check for T
where
    T: FnMut(&u8) -> MatchResult,
{
    fn parse<'a, 'i, IO>(&'a mut self, io: &'i mut BufReader<IO>) -> Fut<'i, io::Result<Vec<u8>>>
    where
        'a: 'i,
        IO: Read + Send + Unpin,
        Self: Send,
    {
        struct Fill<'a, T>(&'a mut BufReader<T>);
        impl<'a, T: Read + Unpin> Future for Fill<'a, T> {
            type Output = io::Result<()>;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                ready!(Pin::new(&mut self.get_mut().0).poll_fill_buf(cx))?;
                Poll::Ready(Ok(()))
            }
        }

        Box::pin(async move {
            let mut result = vec![];
            let mut res = MatchResult::Incomplete;
            let matched = 'grand: loop {
                if io.buffer().is_empty() {
                    Fill(io).await?;
                }
                if io.buffer().is_empty() {
                    // EOF: incomplete => mismatch, match => complete
                    match res {
                        MatchResult::Fail => unreachable!("cannot be"),
                        MatchResult::Mismatch => unreachable!("cannot be"),
                        MatchResult::Complete => unreachable!("cannot be"),
                        MatchResult::Incomplete => break 'grand false,
                        MatchResult::Match => break 'grand true,
                    }
                }
                for i in 0..io.buffer().len() {
                    res = self(&io.buffer()[i]);
                    match res {
                        MatchResult::Fail => return Err(io::ErrorKind::Unsupported.into()),
                        MatchResult::Mismatch => break 'grand false,
                        MatchResult::Complete => {
                            result.extend_from_slice(&io.buffer()[0..i + 1]);
                            Pin::new(&mut *io).consume(i + 1);
                            break 'grand true;
                        }
                        MatchResult::Incomplete => {}
                        MatchResult::Match => {}
                    }
                }
            };

            debug!(
                "Incoming bytes: {:?}",
                String::from_utf8_lossy(result.as_ref())
            );

            if matched {
                Ok(result)
            } else {
                Err(io::ErrorKind::InvalidInput.into())
            }
        })
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum MatchResult {
    Incomplete,
    Match,
    Complete,
    Mismatch,
    Fail,
}

pub fn tag(what: impl fmt::Display) -> impl FnMut(&u8) -> MatchResult {
    let expect = what.to_string().into_bytes();
    let mut pos = 0;
    move |b| {
        if pos == expect.len() {
            MatchResult::Fail
        } else if b == &expect[pos] {
            pos += 1;
            if pos == expect.len() {
                MatchResult::Complete
            } else {
                MatchResult::Incomplete
            }
        } else {
            pos = expect.len();
            MatchResult::Mismatch
        }
    }
}

#[test]
fn tag_completes() {
    let mut sut = tag("123");
    assert_eq!(sut(&b'1'), MatchResult::Incomplete);
    assert_eq!(sut(&b'2'), MatchResult::Incomplete);
    assert_eq!(sut(&b'3'), MatchResult::Complete);
    assert_eq!(sut(&b'4'), MatchResult::Fail);
    assert_eq!(sut(&b'5'), MatchResult::Fail);
}
#[test]
fn tag_mismatch() {
    let mut sut = tag("123");
    assert_eq!(sut(&b'1'), MatchResult::Incomplete);
    assert_eq!(sut(&b'x'), MatchResult::Mismatch);
    assert_eq!(sut(&b'3'), MatchResult::Fail);
    assert_eq!(sut(&b'4'), MatchResult::Fail);
}
#[test]
fn tag_empty() {
    let mut sut = tag("");
    assert_eq!(sut(&b'1'), MatchResult::Fail);
}

#[async_std::test]
async fn parse_tag_matches() {
    let mut inp = "xyz".as_bytes();
    let mut rdr = BufReader::new(&mut inp);
    let rslt = tag("x").parse(&mut rdr).await.expect("ok");
    assert_eq!(rslt, b"x")
}

#[async_std::test]
async fn parse_tag_mismatches() {
    let mut inp = "xyz".as_bytes();
    let mut rdr = BufReader::new(&mut inp);
    let rslt = tag("O").parse(&mut rdr).await.err().expect("err");
    assert_eq!(rslt.kind(), io::ErrorKind::InvalidInput)
}

#[async_std::test]
async fn parse_tag_on_empty_rdr_mismatches() {
    let mut inp = "".as_bytes();
    let mut rdr = BufReader::new(&mut inp);
    let rslt = tag("O").parse(&mut rdr).await.err().expect("err");
    assert_eq!(rslt.kind(), io::ErrorKind::InvalidInput)
}
#[async_std::test]
async fn parse_empty_tag_is_unsupported() {
    let mut inp = "xyz".as_bytes();
    let mut rdr = BufReader::new(&mut inp);
    let rslt = tag("").parse(&mut rdr).await.err().expect("err");
    assert_eq!(rslt.kind(), io::ErrorKind::Unsupported)
}
