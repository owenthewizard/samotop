use std::io;
use std::str;
use bytes::{BytesMut, BufMut, Bytes};
use regex::bytes::Regex;
use tokio_io::codec::{Encoder, Decoder};
use super::parser::SmtpSessionParser;
use super::writer::SmtpAnswerWriter;
use model::request::{SmtpInput, SmtpCommand};
use model::response::SmtpReply;

type Result = io::Result<Option<SmtpInput>>;
type Error = io::Error;

enum InputFlow {
    Stop,
    Continue,
}

pub struct SmtpCodec<'a> {
    requests: Vec<SmtpInput>,
    parser: &'a SmtpSessionParser,
    writer: &'a SmtpAnswerWriter,
    streaming_data: bool,
    stream_pos: usize,
    closed: bool,
    dot_regex: Regex,
}

impl<'a> SmtpCodec<'a> {
    pub fn new(parser: &'a SmtpSessionParser, writer: &'a SmtpAnswerWriter) -> Self {
        Self {
            requests: vec![],
            writer,
            parser,
            streaming_data: false,
            stream_pos: 0,
            closed: false,
            dot_regex: Regex::new(r"\r\n\.\r\n").unwrap(),
        }
    }

    fn dequeue_input(&mut self) -> Result {
        // ToDo: self.requests.remove_item()
        if self.requests.is_empty() {
            Ok(None)
        } else {
            Ok(Some(self.requests.remove(0)))
        }
    }

    fn queue_input(&mut self, inp: SmtpInput) -> InputFlow {
        let inp = inp.pos(self.stream_pos);
        self.stream_pos += inp.len();
        self.requests.push(inp);
        InputFlow::Continue
    }

    fn process_input(&mut self, inp: SmtpInput) -> InputFlow {

        if self.closed {
            panic!("connection has been closed already")
        }

        // TODO: handle ordering situations:
        // * non-Stream... input comes while streaming_data
        // * StreamData or StreamEnd comes while not streaming_data

        match inp {
            SmtpInput::Command(_, _, SmtpCommand::Data) => {
                self.queue_input(inp);
                if !self.streaming_data {
                    // make sure there is StreamStart after Data
                    self.queue_input(SmtpInput::StreamStart(0));
                    self.streaming_data = true;
                }
                InputFlow::Stop
            }
            SmtpInput::StreamStart(_) => {
                if self.streaming_data {
                    // make sure we don't send StreamStart twice
                    InputFlow::Continue
                } else {
                    self.streaming_data = true;
                    self.queue_input(inp)
                }
            }
            SmtpInput::StreamEnd(_) => {
                self.streaming_data = false;
                self.queue_input(inp)
            }
            SmtpInput::Disconnect => {
                self.streaming_data = false;
                self.closed = true;
                self.queue_input(inp)
            }
            SmtpInput::Incomplete(_, _, _) => {
                // data will be returned to the input buffer
                // to be used as a tail for next time round
                InputFlow::Stop
            }
            _ => self.queue_input(inp),
        }
    }

    fn decode_buffer(&mut self, buf: &mut BytesMut) {

        if self.streaming_data {

            // remove all bytes from buffer to avoid ownership issues
            let bytes = buf.take();

            // find the lone dot
            if let Some(dot) = self.dot_regex.find(&bytes[..]) {

                trace!("Got DATA, dot found {} - {}", dot.start(), dot.end());

                // extract the chunk until the lone dot
                self.process_input(SmtpInput::StreamData(
                    0,
                    dot.start(),
                    Bytes::from(&bytes[..dot.start()]),
                ));

                // this will end the body streaming
                self.process_input(SmtpInput::StreamEnd(0));

                // return remaining bytes to buffer
                buf.extend_from_slice(&bytes[dot.end()..]);

            } else {

                trace!("Got DATA, no dot");

                // no dot so all the buffer is a chunk
                self.process_input(SmtpInput::StreamData(
                    0,
                    bytes.len(),
                    Bytes::from(&bytes[..]),
                ));
            }

        } else {
            // not streaming

            let bytes = &buf.take()[..];

            let text = str::from_utf8(bytes);

            trace!("text ({}): {:?}", bytes.len(), text);

            match text {
                Err(e) => {
                    warn!("input error: {:?}, bytes: {:?}", e, bytes);
                    self.process_input(SmtpInput::Invalid(0, bytes.len(), Bytes::from(bytes)));
                }
                Ok(s) => {
                    match self.parser.session(s) {
                        Err(e) => {
                            warn!("parse error: {:?}, text: {:?}", e, text);
                            self.process_input(
                                SmtpInput::Invalid(0, bytes.len(), Bytes::from(bytes)),
                            );
                        }
                        Ok(inputs) => {
                            let parser_offset = self.stream_pos;

                            for inp in inputs {
                                match self.process_input(inp) {
                                    InputFlow::Stop => break,
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
        }
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
                        warn!("unexpected EOF");
                        self.process_input(SmtpInput::Disconnect);
                        self.dequeue_input()
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

        self.dequeue_input()
    }
}

impl<'a> Encoder for SmtpCodec<'a> {
    type Item = SmtpReply;
    type Error = Error;

    fn encode(&mut self, reply: Self::Item, buf: &mut BytesMut) -> io::Result<()> {
        self.writer.write(&mut buf.writer(), reply)
    }
}
