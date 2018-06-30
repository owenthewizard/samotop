use bytes::{BufMut, BytesMut};
use model::request::{SmtpCommand, SmtpInput};
use model::response::SmtpReply;
use protocol::parser::SmtpSessionParser;
use protocol::writer::SmtpAnswerSerializer;
use std::fmt::{Debug, Display};
use std::io;
use std::net::SocketAddr;
use std::str;
use std::time::SystemTime;
use tokio_io::codec::{Decoder, Encoder};

type Result = io::Result<Option<SmtpCommand>>;

pub struct SmtpCodec<'a> {
    requests: Vec<SmtpCommand>,
    parser: &'a SmtpSessionParser,
    serializer: &'a SmtpAnswerSerializer,
    local_addr: Option<SocketAddr>,
    peer_addr: Option<SocketAddr>,
    established: SystemTime,
    initialized: bool,
    closed: bool,
}

impl<'a> SmtpCodec<'a> {
    pub fn new(
        parser: &'a SmtpSessionParser,
        serializer: &'a SmtpAnswerSerializer,
        local_addr: Option<SocketAddr>,
        peer_addr: Option<SocketAddr>,
        established: SystemTime,
    ) -> Self {
        Self {
            requests: vec![],
            serializer,
            parser,
            local_addr,
            peer_addr,
            established,
            initialized: false,
            closed: false,
        }
    }
    fn log(&self, info: &Display) -> String {
        let msg = format!(
            "{}\nLocal: {:?}\nRemote: {:?}",
            info, self.local_addr, self.peer_addr
        );
        println!("{}", msg);
        msg
    }
    fn input_err(&self, e: &Debug, bytes: &[u8]) -> String {
        let msg = format!("input error: {:?}", e);
        self.log(&msg);
        msg
    }
    fn parse_err(&self, e: &Debug, text: &str) -> String {
        let msg = format!("parse error: {:?}", e);
        self.log(&msg);
        format!("{}", text)
    }
    fn eof_err(&self) {
        self.log(&format!("unexpected EOF"));
    }
}

impl<'a> Decoder for SmtpCodec<'a> {
    type Item = SmtpCommand;
    type Error = io::Error;
    fn decode_eof(&mut self, buf: &mut BytesMut) -> Result {
        match try!(self.decode(buf)) {
            Some(frame) => Ok(Some(frame)),
            None => match (buf.is_empty(), self.closed) {
                (false, _) => {
                    Err(io::Error::new(io::ErrorKind::Other, "bytes remaining on stream").into())
                }
                (true, true) => Ok(None),
                (true, false) => {
                    self.closed = true;
                    self.eof_err();
                    Ok(Some(SmtpCommand::Disconnect))
                }
            },
        }
    }
    fn decode(&mut self, buf: &mut BytesMut) -> Result {
        trace!("attempting to decode a frame");

        if !self.initialized {
            trace!(
                "new connection from {:?} to {:?}",
                self.peer_addr,
                self.local_addr
            );

            self.requests.push(SmtpCommand::Connect {
                local_addr: self.local_addr,
                peer_addr: self.peer_addr,
            });

            self.initialized = true;
        }

        if !buf.is_empty() {
            let bytes = &buf.take()[..];

            let text = str::from_utf8(bytes);

            trace!("text ({}): {:?}", bytes.len(), text);

            match text {
                Err(e) => {
                    let s = self.input_err(&e, bytes);
                    self.requests.push(SmtpCommand::Invalid(s));
                }
                Ok(s) => {
                    match self.parser.session(s) {
                        Err(e) => {
                            self.parse_err(&e, s);
                            self.requests.push(SmtpCommand::Invalid(s.to_string()));
                        }
                        Ok(inputs) => {
                            let mut pos = 0;
                            for inp in inputs {
                                match inp {
                                    SmtpInput::Command(b, l, c) => {
                                        pos = b + l;
                                        self.requests.push(c);
                                    }
                                    SmtpInput::None(b, l, _) => {
                                        pos = b + l;
                                    }
                                    SmtpInput::Data(b, l, _) => {
                                        // ToDo handle data properly
                                        pos = b + l;
                                    }
                                    SmtpInput::Incomplete(b, _, _) => {
                                        // data will be returned to the input buffer
                                        pos = b;
                                    }
                                };
                            }

                            // return tail to the input buffer
                            buf.extend_from_slice(&bytes[pos..]);

                            trace!("last position {}, tail {:?}", pos, str::from_utf8(buf));
                        }
                    }
                }
            };
        }

        // ToDo: self.requests.remove_item()
        match self.requests.is_empty() {
            true => Ok(None),
            false => Ok(Some(self.requests.remove(0))),
        }
    }
}

impl<'a> Encoder for SmtpCodec<'a> {
    type Item = SmtpReply;
    type Error = io::Error;

    fn encode(&mut self, reply: Self::Item, buf: &mut BytesMut) -> io::Result<()> {
        self.serializer.write(&mut buf.writer(), reply)
    }
}
