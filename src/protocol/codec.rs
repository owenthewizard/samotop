use std::io;
use std::str;
use std::fmt::Debug;
use bytes::{BytesMut, BufMut};
use tokio_io::codec::{Encoder, Decoder};
use tokio_proto::streaming::pipeline::Frame;
use protocol::parser::SmtpSessionParser;
use protocol::writer::SmtpAnswerSerializer;
use model::request::{SmtpInput, SmtpCommand};
use protocol::{CmdFrame, RplFrame, Error};

type Result = io::Result<Option<CmdFrame>>;

pub struct SmtpCodec<'a> {
    requests: Vec<CmdFrame>,
    parser: &'a SmtpSessionParser,
    serializer: &'a SmtpAnswerSerializer,
    decoding_data: bool,
    closed: bool,
}

impl<'a> SmtpCodec<'a> {
    pub fn new(parser: &'a SmtpSessionParser, serializer: &'a SmtpAnswerSerializer) -> Self {
        Self {
            requests: vec![],
            serializer,
            parser,
            decoding_data: false,
            closed: false,
        }
    }


    fn log(&self, info: &Debug) -> String {
        let msg = format!("{:?}", info);
        println!("{}", msg);
        msg
    }
    fn input_err(&self, e: &Debug, bytes: &[u8]) -> String {
        let msg = format!("input error: {:?}, bytes: {:?}", e, bytes);
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
    type Item = CmdFrame;
    type Error = Error;
    fn decode_eof(&mut self, buf: &mut BytesMut) -> Result {
        match try!(self.decode(buf)) {
            Some(frame) => Ok(Some(frame)),
            None => {
                match (buf.is_empty(), self.closed) {
                    (false, _) => Err(
                        Error::new(io::ErrorKind::Other, "bytes remaining on stream")
                            .into(),
                    ),
                    (true, true) => Ok(None),
                    (true, false) => {
                        self.closed = true;
                        self.eof_err();
                        Ok(Some(Frame::Message {
                            body: false,
                            message: SmtpCommand::Disconnect,
                        }))
                    }
                }
            }
        }
    }
    fn decode(&mut self, buf: &mut BytesMut) -> Result {
        println!("attempting to decode a frame");

        if !buf.is_empty() {

            let bytes = &buf.take()[..];

            let text = str::from_utf8(bytes);

            println!("text ({}): {:?}", bytes.len(), text);

            match text {
                Err(e) => {
                    let s = self.input_err(&e, bytes);
                    let f = Frame::Message {
                        body: false,
                        message: SmtpCommand::Invalid(s),
                    };
                    self.requests.push(f);
                }
                Ok(s) => {
                    match self.parser.session(s) {
                        Err(e) => {
                            self.parse_err(&e, s);
                            let f = Frame::Message {
                                body: false,
                                message: SmtpCommand::Invalid(s.to_string()),
                            };
                            self.requests.push(f);
                        }
                        Ok(inputs) => {
                            let mut pos = 0;
                            for inp in inputs {
                                match inp {
                                    SmtpInput::Command(b, l, c) => {
                                        pos = b + l;
                                        let f = Frame::Message {
                                            body: false,
                                            message: c,
                                        };
                                        self.requests.push(f);
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

                            println!("last position {}, tail {:?}", pos, str::from_utf8(buf));
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
    type Item = RplFrame;
    type Error = Error;

    fn encode(&mut self, frame: Self::Item, buf: &mut BytesMut) -> io::Result<()> {
        match frame {
            Frame::Message {
                message: reply,
                body: _,
            } => self.serializer.write(&mut buf.writer(), reply),
            Frame::Body { .. } => Err(io::Error::new(
                io::ErrorKind::Other,
                "streaming reply not supported",
            )),
            e @ Frame::Error { .. } => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("error frame: {:?}", e),
            )),
        }
    }
}
