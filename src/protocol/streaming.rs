use std::io;
use bytes::{Bytes, BytesMut};
use tokio_io::codec::{Framed, Decoder, Encoder};
use tokio_proto::streaming::pipeline::{ServerProto, Frame};
use protocol::socket::NetSocket;
use model::request::SmtpCommand;
use model::response::*;

pub struct SmtpProto;

impl<T: NetSocket + 'static> ServerProto<T> for SmtpProto {
    type Request = SmtpCommand;
    type RequestBody = Bytes;
    type Response = SmtpReply;
    type ResponseBody = Bytes;
    type Error = io::Error;
    type Transport = Framed<T, SmtpCodec>;
    type BindTransport = Result<Self::Transport, io::Error>;

    fn bind_transport(&self, io: T) -> Self::BindTransport {
        let codec = SmtpCodec::new();
        Ok(io.framed(codec))
    }
}

pub struct SmtpCodec {
    decoding_data: bool,
}

impl SmtpCodec {
    pub fn new() -> Self {
        Self { decoding_data: false }
    }
}

impl Decoder for SmtpCodec {
    type Item = Frame<SmtpCommand, Bytes, io::Error>;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, io::Error> {
        buf.clear();

        Ok(Some(Frame::Message {
            message: SmtpCommand::Unknown("demo".to_string()),
            body: false,
        }))

        /*
        // Find the position of the next newline character and split off the
        // line if we find it.
        let line = match buf.iter().position(|b| *b == b'\n') {
            Some(n) => buf.split_to(n),
            None => return Ok(None),
        };

        // Also remove the '\n'
        buf.split_to(1);

        // Turn this data into a string and return it in a Frame
        let s = try!(str::from_utf8(&line).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, e)
        }));

        // Got an empty line, which means that the state
        // should be toggled.
        if s == "" {
            let decoding_data = self.decoding_data;
            // Toggle the state
            self.decoding_data = !decoding_data;

            if decoding_data {
                Ok(Some(Frame::Message {
                    // The message head is an empty line
                    message: s.to_string(),
                    // We will be streaming a body next
                    body: true,
                }))
            } else {
                // The streaming body termination frame,
                // is represented as `None`.
                Ok(Some(Frame::Body {
                    chunk: None
                }))
            }
        } else {
            if self.decoding_data {
                // This is a "oneshot" message with no
                // streaming body
                Ok(Some(Frame::Message {
                    message: s.to_string(),
                    body: false,
                }))
            } else {
                // Streaming body line chunk
                Ok(Some(Frame::Body {
                    chunk: Some(s.to_string()),
                }))
            }
        }
        */
    }
}

impl Encoder for SmtpCodec {
    type Item = Frame<SmtpReply, Bytes, io::Error>;
    type Error = io::Error;

    fn encode(&mut self, msg: Self::Item, buf: &mut BytesMut) -> io::Result<()> {
        match msg {
            Frame::Message { message, body } => {
                buf.extend(format!("{:?}", message).as_bytes());
            }
            Frame::Body { chunk } => {
                if let Some(chunk) = chunk {
                    buf.extend(format!("{:?}", chunk).as_bytes());
                }
            }
            Frame::Error { error } => {
                // Our protocol does not support error frames, so
                // this results in a connection level error, which
                // will terminate the socket.
                return Err(error);
            }
        }

        // Push the new line
        buf.extend(b"\n");

        Ok(())
    }
}
