use std::io;
use std::str;
use std::fmt::Debug;
use bytes::{BytesMut, BufMut, Bytes};
use regex::bytes::Regex;
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
    streaming_data: bool,
    closed: bool,
    dot_regex: Regex,
}

impl<'a> SmtpCodec<'a> {
    pub fn new(parser: &'a SmtpSessionParser, serializer: &'a SmtpAnswerSerializer) -> Self {
        Self {
            requests: vec![],
            serializer,
            parser,
            streaming_data: false,
            closed: false,
            dot_regex: Regex::new(r"\r\n\.\r\n").unwrap(),
        }
    }

    fn err(&self, err: &str) {
        warn!("{}", err)
    }
    fn input_err(&self, e: &Debug, bytes: &[u8]) -> String {
        let msg = format!("input error: {:?}, bytes: {:?}", e, bytes);
        self.err(&msg);
        msg
    }
    fn parse_err(&self, e: &Debug, text: &str) {
        let msg = format!("parse error: {:?}, text: {:?}", e, text);
        self.err(&msg);
    }
    fn eof_err(&self) {
        self.err(&format!("unexpected EOF"));
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
        trace!("attempting to decode a frame");

        // TODO: Check buffer work efficiency, reduce copies if possible

        if !buf.is_empty() {

            if self.streaming_data {

                // remove all bytes from buffer to avoid ownership issues
                let bytes = buf.take();

                // find the lone dot
                if let Some(dot) = self.dot_regex.find(&bytes[..]) {

                    // dot found so we'll finish streaming
                    self.streaming_data = false;

                    trace!("Got DATA, dot found {} - {}", dot.start(), dot.end());

                    // extract the chunk until the lone dot
                    self.requests.push(Frame::Body {
                        chunk: Some(Bytes::from(&bytes[..dot.start()])),
                    });
                    // this will end the body streaming
                    self.requests.push(Frame::Message {
                        body: false,
                        message: SmtpCommand::EndOfStream,
                    });

                    // return remaining bytes to buffer
                    buf.extend_from_slice(&bytes[dot.end()..]);

                } else {

                    trace!("Got DATA, no dot");

                    // no dot so all the buffer is a chunk
                    self.requests.push(Frame::Body {
                        chunk: Some(Bytes::from(&bytes[..])),
                    });
                }

            } else {

                let bytes = &buf.take()[..];

                let text = str::from_utf8(bytes);

                trace!("text ({}): {:?}", bytes.len(), text);

                match text {
                    Err(e) => {
                        let s = self.input_err(&e, bytes);
                        self.requests.push(Frame::Message {
                            body: false,
                            //TODO: Pass bytes
                            message: SmtpCommand::Unknown(s),
                        });
                    }
                    Ok(s) => {
                        match self.parser.session(s) {
                            Err(e) => {
                                self.parse_err(&e, s);
                                self.requests.push(Frame::Message {
                                    body: false,
                                    //TODO: Pass bytes
                                    message: SmtpCommand::Unknown(s.to_string()),
                                });
                            }
                            Ok(inputs) => {
                                let mut pos = 0;
                                for inp in inputs {
                                    match inp {
                                        i @ SmtpInput::Connect(_) => panic!(),
                                        i @ SmtpInput::Disconnect => panic!(),
                                        SmtpInput::Command(b, l, c @ SmtpCommand::Data) => {
                                            pos = b + l;
                                            self.requests.push(Frame::Message {
                                                body: false,
                                                message: c,
                                            });
                                            self.requests.push(Frame::Message {
                                                body: true,
                                                message: SmtpCommand::Stream,
                                            });
                                            self.streaming_data = true;
                                            break;
                                        }
                                        SmtpInput::Command(b, l, c) => {
                                            pos = b + l;
                                            self.requests.push(Frame::Message {
                                                body: false,
                                                message: c,
                                            });
                                        }
                                        SmtpInput::None(b, l, _) => {
                                            pos = b + l;
                                        }
                                        SmtpInput::StreamStart(b) => (),
                                        SmtpInput::StreamEnd(b) => (),
                                        SmtpInput::StreamData(b, l, _) => {
                                            // ToDo handle data properly if it comes
                                            pos = b + l;
                                        }
                                        SmtpInput::Invalid(b, l, s) => {
                                            match s.ends_with("\n") {
                                                true => {
                                                    pos = b + l;
                                                    self.requests.push(Frame::Message {
                                                        body: false,
                                                        message: SmtpCommand::Unknown(s),
                                                    });
                                                }
                                                false => {
                                                    // data will be returned to the input buffer
                                                    // to be used as a tail for next time round
                                                    pos = b;
                                                }
                                            }
                                        }
                                        SmtpInput::InvalidBytes(b, l, d) => {
                                            match d.ends_with(b"\n") {
                                                true => {
                                                    pos = b + l;
                                                    self.requests.push(Frame::Message {
                                                        body: false,
                                                        message: SmtpCommand::Unknown(
                                                            str::from_utf8(&d[..])
                                                                .unwrap()
                                                                .to_owned(),
                                                        ),
                                                    });
                                                }
                                                false => {
                                                    // data will be returned to the input buffer
                                                    // to be used as a tail for next time round
                                                    pos = b;
                                                }
                                            }
                                        }
                                    };
                                }

                                // return tail to the input buffer
                                buf.extend_from_slice(&bytes[pos..]);

                                trace!("last position {}, tail {:?}", pos, str::from_utf8(buf));
                            }
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
