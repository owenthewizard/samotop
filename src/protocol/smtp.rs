use bytes::{BufMut, Bytes, BytesMut};
use model::command::SmtpCommand;
use model::controll::*;
use regex::bytes::Regex;
use tokio::io;
use tokio_codec::{Decoder, Encoder};

/**
 * Low level SMTP codec that handles command lines and data
 * on top of a raw byte stream
 *
 * Simple rule: return as soon as we have something certain
 * and return as much as we can of it. Keep uncertain bytes in the buffer.
 */
pub struct SmtpCodec {
    nl_lookup: Regex,
    //data_check: Regex,
    sanity_check: Regex,
    state: State,
}

/** Tracks the codec state */
enum State {
    /** Parsing command lines, (head and tail positions) */
    Line(usize, usize),
    /** Parsing data, (new line, position) */
    Data(bool, usize),
}

impl SmtpCodec {
    pub fn new() -> Self {
        Self {
            nl_lookup: Regex::new(r"\r?\n|\r$").unwrap(),
            // performs basic sanity check on a command line
            sanity_check: Regex::new(r"(?i)^[a-z]{4}(\r?\n| )").unwrap(),
            state: State::Line(0, 0),
            // we could detect data command in the codec
            // but it would not give the service a chance to decide
            //data_check: Regex::new(r"(?i)^data(\r?\n| )").unwrap(),
        }
    }

    pub fn decode_either(
        &mut self,
        buf: &mut BytesMut,
    ) -> Result<Option<ServerControll>, io::Error> {
        let (ctrl, state) = match self.state {
            State::Data(nl, pos) => self.decode_data(buf, nl, pos),
            State::Line(_, tail) => self.decode_line(buf, tail),
        };
        self.adjust(buf, state);
        ctrl
    }
    fn decode_line(
        &mut self,
        buf: &mut BytesMut,
        tail: usize,
    ) -> (Result<Option<ServerControll>, io::Error>, State) {
        // find next new line after tail
        match self.nl_lookup.find_at(&buf.as_ref()[..], tail) {
            None => (
                // no new line was found
                Ok(None),
                // advance the tail for next round and carry on
                State::Line(0, buf.len()),
            ),
            Some(found) => {
                let tail = found.end();
                // Split the buffer at the index of the '\n' + 1 to include the '\n'.
                // `split_to` returns a new buffer with the contents up to the index.
                // The buffer on which `split_to` is called will now start at this index.
                let bytes = &buf[0..tail];

                // advance to the tail
                let state = State::Line(tail, tail);

                if !self.sanity_check.is_match(bytes) {
                    (Ok(Some(ServerControll::Invalid(Bytes::from(bytes)))), state)
                } else {
                    //if self.data_check.is_match(bytes) {
                    //    state = State::Data(true, tail)
                    //}

                    // Convert the bytes to a string and panic if the bytes are not valid utf-8.
                    let line = String::from_utf8(bytes.to_vec());

                    // Return Ok(Some(...)) to signal that a full frame has been produced.
                    match line {
                        Err(_) => (Ok(Some(ServerControll::Invalid(Bytes::from(bytes)))), state),
                        Ok(line) => (
                            Ok(Some(ServerControll::Command(SmtpCommand::Unknown(line)))),
                            state,
                        ),
                    }
                }
            }
        }
    }

    fn decode_data(
        &mut self,
        buf: &mut BytesMut,
        nl: bool,
        pos: usize,
    ) -> (Result<Option<ServerControll>, io::Error>, State) {
        if buf.len() == 0 {
            return (Ok(None), State::Data(nl, pos));
        }
        use self::DotState::*;
        match dotstate(&mut buf.iter(), nl) {
            Wait => (Ok(None), State::Data(nl, pos)),
            End(end) => (
                // it is the data terminating line
                Ok(Some(ServerControll::FinalDot(Bytes::from(&buf[..end])))),
                State::Line(end, end),
            ),
            Escape(0) => (
                // the first byte is an escaping dot, send just the dot
                Ok(Some(ServerControll::EscapeDot(Bytes::from(&buf[..1])))),
                State::Data(false, 1),
            ),
            Escape(pos) => (
                // there is an escaping dot at pos, send data before pos
                Ok(Some(ServerControll::DataChunk(Bytes::from(&buf[..pos])))),
                State::Data(true, pos),
            ),
            LF => (
                Ok(Some(ServerControll::DataChunk(Bytes::from(&b"\n"[..])))),
                State::Data(true, 1),
            ),
            CRLF => (
                Ok(Some(ServerControll::DataChunk(Bytes::from(&b"\r\n"[..])))),
                State::Data(true, 2),
            ),
            GoOn => match self.nl_lookup.find(&buf.as_ref()[..]) {
                Some(found) => (
                    Ok(Some(ServerControll::DataChunk(Bytes::from(
                        &buf[..found.start()],
                    )))),
                    State::Data(false, found.start()),
                ),
                None => (
                    Ok(Some(ServerControll::DataChunk(Bytes::from(&buf[..])))),
                    State::Data(false, buf.len()),
                ),
            },
        }
    }

    fn adjust(&mut self, buf: &mut BytesMut, state: State) {
        self.state = match state {
            State::Line(head, tail) => {
                assert!(tail <= buf.len());
                assert!(head <= tail);
                let _ = buf.split_to(head);
                let tail = tail - head;
                let head = 0;
                State::Line(head, tail)
            }
            State::Data(nl, pos) => {
                assert!(pos <= buf.len());
                let _ = buf.split_to(pos);
                let pos = 0;
                State::Data(nl, pos)
            }
        }
    }
}

impl Decoder for SmtpCodec {
    type Item = ServerControll;
    type Error = io::Error;
    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<ServerControll>, io::Error> {
        self.decode_either(buf)
    }
}

impl Encoder for SmtpCodec {
    type Item = ClientControll;
    type Error = io::Error;
    fn encode(&mut self, item: Self::Item, buf: &mut BytesMut) -> Result<(), Self::Error> {
        let line = match item {
            ClientControll::Noop => return Ok(()),
            ClientControll::Shutdown => return Ok(()),
            ClientControll::AcceptData => {
                if let State::Line(_, tail) = self.state {
                    self.state = State::Data(true, tail);
                }
                return Ok(());
            }
            ClientControll::Reply(reply) => reply.to_string(),
        };

        // It's important to reserve the amount of space needed. The `bytes` API
        // does not grow the buffers implicitly.
        // Reserve the length of the string + 1 for the '\n'.
        buf.reserve(line.len());

        // String implements IntoBuf, a trait used by the `bytes` API to work with
        // types that can be expressed as a sequence of bytes.
        buf.put(line);

        // Return ok to signal that no error occured.
        Ok(())
    }
}

#[derive(Eq, PartialEq, Debug)]
pub enum DotState {
    /** Need more bytes */
    Wait,
    /** Data ending dot has been found (\r\n.\r\n => 5) */
    End(usize),
    /** Escaping dot has been found at position (\r\n..\r\n => 2) */
    Escape(usize),
    /** Line feed was found and can be consumed => nl */
    LF,
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
                Some(b'\n') => End(2),
                Some(b'\r') => match iter.next() {
                    None => Wait,
                    Some(b'\n') => End(3),
                    Some(_) => Escape(0),
                },
                Some(_) => Escape(0),
            },
            (true, b'\n') => LF,
            (true, b'\r') => match iter.next() {
                None => Wait,
                Some(b'\n') => CRLF,
                Some(_) => GoOn,
            },
            (true, _) => GoOn,
            (false, b'\n') => match iter.next() {
                None => Wait,
                Some(b'.') => match iter.next() {
                    None => Wait,
                    Some(b'\n') => End(3),
                    Some(b'\r') => match iter.next() {
                        None => Wait,
                        Some(b'\n') => End(4),
                        Some(_) => Escape(1),
                    },
                    Some(_) => Escape(1),
                },
                Some(_) => LF,
            },
            (false, b'\r') => match iter.next() {
                None => Wait,
                Some(b'\n') => match iter.next() {
                    None => Wait,
                    Some(b'.') => match iter.next() {
                        None => Wait,
                        Some(b'\n') => End(4),
                        Some(b'\r') => match iter.next() {
                            None => Wait,
                            Some(b'\n') => End(5),
                            Some(_) => Escape(2),
                        },
                        Some(_) => Escape(2),
                    },
                    Some(_) => CRLF,
                },
                Some(_) => GoOn,
            },
            (false, _) => GoOn,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::DotState::*;
    use protocol::smtp::dotstate;

    #[test]
    fn dotstate_handles_empty_line() {
        let r = dotstate(&mut b".\r\n".iter(), true);
        assert_eq!(r, End(3));
        let r = dotstate(&mut b"\n.\n".iter(), false);
        assert_eq!(r, End(3));
        let r = dotstate(&mut b"\r\n.\r\n".iter(), false);
        assert_eq!(r, End(5));
    }

    #[test]
    fn dotstate_handles_escape_dot() {
        let r = dotstate(&mut b"..\r\n".iter(), true);
        assert_eq!(r, Escape(0));
        let r = dotstate(&mut b".xxx\r\n".iter(), true);
        assert_eq!(r, Escape(0));
        let r = dotstate(&mut b"\r\n..\r\n".iter(), false);
        assert_eq!(r, Escape(2));
        let r = dotstate(&mut b"\r\n.xxx\r\n".iter(), false);
        assert_eq!(r, Escape(2));
        let r = dotstate(&mut b"\n..\n".iter(), false);
        assert_eq!(r, Escape(1));
        let r = dotstate(&mut b"\n.xxx\n".iter(), false);
        assert_eq!(r, Escape(1));
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
        assert_eq!(r, Wait);
        let r = dotstate(&mut b"\r\n.".iter(), false);
        assert_eq!(r, Wait);
        let r = dotstate(&mut b"\r\n.\r".iter(), false);
        assert_eq!(r, Wait);
    }

}
