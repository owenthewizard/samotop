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
use model::response::SmtpReply;
use protocol::{CmdFrame, RplFrame, Error};

type Result = io::Result<Option<SmtpInput>>;

enum InputFlow {
    Stop,
    Continue,
}

pub struct SmtpCodec<'a> {
    requests: Vec<SmtpInput>,
    parser: &'a SmtpSessionParser,
    serializer: &'a SmtpAnswerSerializer,
    streaming_data: bool,
    stream_pos: usize,
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
            stream_pos: 0,
            closed: false,
            dot_regex: Regex::new(r"\r\n\.\r\n").unwrap(),
        }
    }

    fn process_input(&mut self, inp: SmtpInput) -> InputFlow {
        match inp {
            i @ SmtpInput::Connect(_) => self.requests.push(i),
            i @ SmtpInput::Disconnect => self.requests.push(i),
            SmtpInput::Command(b, l, c @ SmtpCommand::Data) => {
                self.requests.push(
                    SmtpInput::Command(self.stream_pos, l, c),
                );
                self.stream_pos += l;
                if !self.streaming_data {
                    self.requests.push(SmtpInput::StreamStart(self.stream_pos));
                    self.streaming_data = true;
                }
                return InputFlow::Stop;
            }
            SmtpInput::Command(b, l, c) => {
                self.requests.push(
                    SmtpInput::Command(self.stream_pos, l, c),
                );
                self.stream_pos += l;
            }
            SmtpInput::None(b, l, d) => {
                self.requests.push(SmtpInput::None(self.stream_pos, l, d));
                self.stream_pos += l;
            }
            SmtpInput::StreamStart(b) => {
                self.requests.push(SmtpInput::StreamStart(self.stream_pos));
            }
            SmtpInput::StreamEnd(b) => {
                self.requests.push(SmtpInput::StreamEnd(self.stream_pos));
            }
            SmtpInput::StreamData(b, l, d) => {
                self.requests.push(
                    SmtpInput::StreamData(self.stream_pos, l, d),
                );
                self.stream_pos += l;
            }
            SmtpInput::Invalid(b, l, s) => {
                match s.ends_with("\n") {
                    true => {
                        self.requests.push(
                            SmtpInput::Invalid(self.stream_pos, l, s),
                        );
                        self.stream_pos += l;
                    }
                    _ => {
                        // data will be returned to the input buffer
                        // to be used as a tail for next time round
                        ()
                    }
                }
            }
            SmtpInput::InvalidBytes(b, l, d) => {
                match d.ends_with(b"\n") {
                    true => {
                        self.requests.push(
                            SmtpInput::InvalidBytes(self.stream_pos, l, d),
                        );
                        self.stream_pos += l;
                    }
                    _ => {
                        // data will be returned to the input buffer
                        // to be used as a tail for next time round
                        ()
                    }
                }
            }
        };
        InputFlow::Continue
    }

    fn decode_buffer(&mut self, buf: &mut BytesMut) {

        if self.streaming_data {

            // remove all bytes from buffer to avoid ownership issues
            let bytes = buf.take();

            // find the lone dot
            if let Some(dot) = self.dot_regex.find(&bytes[..]) {

                // dot found so we'll finish streaming
                self.streaming_data = false;

                trace!("Got DATA, dot found {} - {}", dot.start(), dot.end());

                // extract the chunk until the lone dot
                self.requests.push(SmtpInput::StreamData(
                    self.stream_pos,
                    dot.start(),
                    Bytes::from(&bytes[..dot.start()]),
                ));

                self.stream_pos += dot.start();

                // this will end the body streaming
                self.requests.push(SmtpInput::StreamEnd(self.stream_pos));

                // return remaining bytes to buffer
                buf.extend_from_slice(&bytes[dot.end()..]);


            } else {

                trace!("Got DATA, no dot");

                // no dot so all the buffer is a chunk
                self.requests.push(SmtpInput::StreamData(
                    self.stream_pos,
                    bytes.len(),
                    Bytes::from(&bytes[..]),
                ));

                self.stream_pos += bytes.len();
            }

        } else {

            let parser_offset = self.stream_pos;
            let bytes = &buf.take()[..];

            let text = str::from_utf8(bytes);

            trace!("text ({}): {:?}", bytes.len(), text);

            match text {
                Err(e) => {
                    warn!("input error: {:?}, bytes: {:?}", e, bytes);
                    self.requests.push(SmtpInput::InvalidBytes(
                        self.stream_pos,
                        bytes.len(),
                        Bytes::from(bytes),
                    ));
                    self.stream_pos += bytes.len();
                }
                Ok(s) => {
                    match self.parser.session(s) {
                        Err(e) => {
                            warn!("parse error: {:?}, text: {:?}", e, text);
                            self.requests.push(SmtpInput::InvalidBytes(
                                self.stream_pos,
                                bytes.len(),
                                Bytes::from(bytes),
                            ));
                            self.stream_pos += bytes.len();
                        }
                        Ok(inputs) => {
                            for inp in inputs {
                                match self.process_input(inp) {
                                    Stop => break,
                                    _ => (),
                                }
                            }

                            // return leftover tail to the input buffer
                            buf.extend_from_slice(&bytes[self.stream_pos - parser_offset..]);

                            trace!(
                                "last position {}, tail {:?}",
                                self.stream_pos,
                                str::from_utf8(buf)
                            );
                        }
                    }
                }
            }
        };
    }
}

impl<'a> Decoder for SmtpCodec<'a> {
    type Item = SmtpInput;
    type Error = Error;
    fn decode_eof(&mut self, buf: &mut BytesMut) -> Result {
        match try!(self.decode(buf)) {
            Some(input) => Ok(Some(input)),
            None => {
                match (buf.is_empty(), self.closed) {
                    (false, _) => Err(Error::new(
                        io::ErrorKind::Other,
                        "bytes remaining on stream",
                    )),
                    (true, true) => Ok(None),
                    (true, false) => {
                        self.closed = true;
                        warn!("unexpected EOF");
                        Ok(Some(SmtpInput::Disconnect))
                    }
                }
            }
        }
    }
    fn decode(&mut self, buf: &mut BytesMut) -> Result {
        trace!("attempting to decode a frame");

        // TODO: Check buffer work efficiency, reduce copies if possible

        if !buf.is_empty() {
            self.decode_buffer(buf);
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
    type Error = Error;

    fn encode(&mut self, reply: Self::Item, buf: &mut BytesMut) -> io::Result<()> {
        self.serializer.write(&mut buf.writer(), reply)
    }
}
