use bytes::{BufMut, Bytes, BytesMut};
use model::command::SmtpCommand;
use model::controll::*;
use tokio::io;
use tokio_codec::{Decoder, Encoder};

pub struct LineCodec {
    next_index: usize,
}

impl LineCodec {
    pub fn new() -> Self {
        LineCodec { next_index: 0 }
    }
}

impl Decoder for LineCodec {
    type Item = ServerControll;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<ServerControll>, io::Error> {
        // Look for a byte with the value '\n' in buf. Start searching from the search start index.
        if let Some(newline_offset) = buf[self.next_index..].iter().position(|b| *b == b'\n') {
            // Found a '\n' in the string.

            // The index of the '\n' is at the sum of the start position + the offset found.
            let newline_index = newline_offset + self.next_index;

            // Split the buffer at the index of the '\n' + 1 to include the '\n'.
            // `split_to` returns a new buffer with the contents up to the index.
            // The buffer on which `split_to` is called will now start at this index.
            let bytes = buf.split_to(newline_index + 1);

            // Convert the bytes to a string and panic if the bytes are not valid utf-8.
            let line = String::from_utf8(bytes.to_vec());

            // Set the search start index back to 0.
            self.next_index = 0;

            // Return Ok(Some(...)) to signal that a full frame has been produced.
            match line {
                Ok(line) => Ok(Some(ServerControll::Command(SmtpCommand::Unknown(line)))),
                Err(_) => Ok(Some(ServerControll::Invalid(Bytes::from(bytes)))),
            }
        } else {
            // '\n' not found in the string.

            // Tell the next call to start searching after the current length of the buffer
            // since all of it was scanned and no '\n' was found.
            self.next_index = buf.len();

            // Ok(None) signifies that more data is needed to produce a full frame.
            Ok(None)
        }
    }
}

impl Encoder for LineCodec {
    type Item = ClientControll;
    type Error = io::Error;
    fn encode(&mut self, item: Self::Item, buf: &mut BytesMut) -> Result<(), Self::Error> {
        let line = match item {
            ClientControll::Noop => return Ok(()),
            ClientControll::Shutdown => return Ok(()),
            ClientControll::AcceptData => return Ok(()),
            ClientControll::QueueMail => return Ok(()),
            ClientControll::Reply(line) => line.to_string(),
        };

        // It's important to reserve the amount of space needed. The `bytes` API
        // does not grow the buffers implicitly.
        // Reserve the length of the string + 1 for the '\n'.
        buf.reserve(line.len() + 1);

        // String implements IntoBuf, a trait used by the `bytes` API to work with
        // types that can be expressed as a sequence of bytes.
        buf.put(line);

        // Return ok to signal that no error occured.
        Ok(())
    }
}
