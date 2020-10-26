use crate::smtp::authentication::Authentication;
use crate::smtp::commands::*;
use crate::smtp::error::{Error, SmtpResult};
use crate::smtp::extension::{ClientId, ServerInfo};
use crate::smtp::response::parse_response;
use crate::smtp::response::Response;
use async_std::io::{self, Read, ReadExt, Write};
use bytes::{Buf, BufMut, BytesMut};
use futures::io::AsyncWriteExt as WriteExt;
use futures::Future;
use log::debug;
use std::fmt::Display;
use std::pin::Pin;
use std::time::Duration;

/// Basic SMTP mail protocol client
/// As a rule of thumb, this code only takes care of SMTP.
/// No encryption or connection setup. Separating concerns.
/// It wraps lightly around the provided stream
/// to facilitate the execution of an SMTP session.
#[derive(Debug)]
pub struct SmtpProto<'s, S> {
    stream: Pin<&'s mut S>,
    buffer: BytesMut,
    line_limit: usize,
}

impl<'s, S> SmtpProto<'s, S> {
    pub fn new(stream: Pin<&'s mut S>) -> Self {
        SmtpProto {
            stream,
            buffer: BytesMut::new(),
            line_limit: 8000,
        }
    }
    // pub fn with_line_limit(mut self, limit: usize) -> Self {
    //     self.line_limit = limit;
    //     self
    // }
    pub fn buffer(&self) -> &[u8] {
        self.buffer.bytes()
    }
    pub fn stream_mut(&mut self) -> Pin<&mut S> {
        self.stream.as_mut()
    }
    pub fn stream(&self) -> Pin<&S> {
        self.stream.as_ref()
    }
    // pub fn into_stream(self) -> Pin<&'s mut S> {
    //     self.stream
    // }
}
impl<'s, S: Read + Write> SmtpProto<'s, S> {
    /// Gets the server banner after connection.
    pub async fn read_banner(&mut self, timeout: Duration) -> SmtpResult {
        let banner_response = self.read_response(timeout).await?;
        banner_response.is([220].as_ref())
    }
    /// Gets the server response after mail data have been fully sent.
    pub async fn read_data_sent_response(&mut self, timeout: Duration) -> SmtpResult {
        let banner_response = self.read_response(timeout).await?;
        banner_response.is([250].as_ref())
    }
    /// Gets the EHLO response and updates server information.
    pub async fn execute_ehlo(
        &mut self,
        me: ClientId,
        timeout: Duration,
    ) -> Result<(Response, ServerInfo), Error> {
        // Extended Hello
        // TODO: Try HELO as a fallback!
        let ehlo_response = self
            .execute_command(EhloCommand::new(me), [250], timeout)
            .await?;
        // extract extensions
        let server_info = ServerInfo::from_response(&ehlo_response)?;
        // Print server information
        debug!("ehlo server info {}", server_info);

        Ok((ehlo_response, server_info))
    }
    /// Sends STARTTLS, and confirms success message. Does not switch protocols!
    /// Do that through the self.stream_mut() or self.into_inner()
    pub async fn execute_starttls(&mut self, timeout: Duration) -> SmtpResult {
        let response = self.execute_command(StarttlsCommand, [220], timeout).await;
        response
    }
    // /// Sends the rset command
    // pub async fn execute_rset(&mut self, timeout: Duration) -> SmtpResult {
    //     let response = self.execute_command(RsetCommand, [250], timeout).await;
    //     response
    // }
    /// Sends the quit command
    pub async fn execute_quit(&mut self, timeout: Duration) -> SmtpResult {
        let response = self.execute_command(QuitCommand, [221], timeout).await;
        response
    }
    /// Sends an AUTH command with the given mechanism, and handles challenge if needed
    pub async fn authenticate<A: Authentication>(
        &mut self,
        mut authentication: A,
        timeout: Duration,
    ) -> SmtpResult {
        // TODO
        let mut challenges = 10u8;
        let mut response = self
            .execute_command(AuthCommand::new(&mut authentication)?, [334, 2], timeout)
            .await?;

        while challenges > 0 && response.has_code(334) {
            challenges -= 1;
            response = self
                .execute_command(
                    AuthResponse::new(&mut authentication, &response)?,
                    [334, 2],
                    timeout,
                )
                .await?;
        }

        if challenges == 0 {
            Err(Error::ResponseParsing("Unexpected number of challenges"))
        } else {
            Ok(response)
        }
    }
    pub async fn execute_command<C: Display, E: AsRef<[u16]>>(
        &mut self,
        command: C,
        expected: E,
        timeout: Duration,
    ) -> SmtpResult {
        let command = command.to_string();
        debug!("C: {}", escape_crlf(command.as_str()));
        let buff = command.as_bytes();
        let written = self.write_bytes(buff, timeout).await?;
        debug_assert!(written == buff.len(), "Make sure we write all the data");
        self.stream.flush().await?;
        let response = self.read_response(timeout).await?;
        response.is(expected)
    }
    async fn write_bytes(&mut self, buf: &[u8], timeout: Duration) -> Result<usize, Error> {
        with_timeout(timeout, self.stream.write(buf)).await
    }
    async fn read_response(&mut self, timeout: Duration) -> SmtpResult {
        with_timeout(timeout, async move {
            loop {
                self.buffer.reserve(1024);
                let buf = self.buffer.bytes_mut();
                // It is OK to use uninitialized buffer as long as read fulfills the contract.
                // In other words, it will only use the given buffer for writing.
                // TODO: What's the story with clippy::transmute-ptr-to-ptr?
                #[allow(unsafe_code)]
                #[allow(clippy::transmute_ptr_to_ptr)]
                let buf = unsafe { std::mem::transmute(buf) };
                let read = self.stream.read(buf).await?;
                if read == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("incomplete after {} bytes", self.buffer().len()),
                    )
                    .into());
                }
                // It is OK to use uninitialized buffer as long as read fulfills the contract.
                // In other words, read bytes have been written at the beginning of the given buffer
                #[allow(unsafe_code)]
                unsafe {
                    self.buffer.advance_mut(read)
                };
                let response = std::str::from_utf8(self.buffer.bytes())?;
                debug!("S: {}", escape_crlf(response));
                break match parse_response(response) {
                    Ok((remaining, response)) => {
                        let consumed = self.buffer.remaining() - remaining.len();
                        self.buffer.advance(consumed);
                        response.is([2, 3].as_ref())
                    }
                    Err(nom::Err::Incomplete(_)) => {
                        // read more unless there's too much
                        if self.buffer.remaining() >= self.line_limit {
                            Err(Error::ResponseParsing("Line limit reached"))
                        } else {
                            continue;
                        }
                    }
                    Err(nom::Err::Failure(e)) => Err(Error::Parsing(e.1)),
                    Err(nom::Err::Error(e)) => Err(Error::Parsing(e.1)),
                };
            }
        })
        .await
    }
}

/// Execute io operations with a timeout.
async fn with_timeout<T, F, E, EOut>(timeout: Duration, f: F) -> std::result::Result<T, EOut>
where
    F: Future<Output = std::result::Result<T, E>>,
    EOut: From<async_std::future::TimeoutError>,
    EOut: From<E>,
{
    let res = async_std::future::timeout(timeout, f).await??;
    Ok(res)
}

/// Returns the string replacing all the CRLF with "\<CRLF\>"
/// Used for debug displays
fn escape_crlf(string: &str) -> String {
    string.replace("\r\n", "<CRLF>")
}
