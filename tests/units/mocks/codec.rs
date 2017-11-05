use std::io;
use std::sync::mpsc::{Sender, Receiver, channel};
use samotop::codec::SmtpAnswerWriter;
use samotop::codec::SmtpSessionParser;
use samotop::codec::ParseResult;
use samotop::model::request::SmtpInput;
use samotop::model::response::SmtpReply;

pub struct MockParser {
    input: Receiver<ParseResult<Vec<SmtpInput>>>,
}

impl MockParser {
    pub fn setup() -> (Self, Sender<ParseResult<Vec<SmtpInput>>>) {
        let (tx_inp, rx_inp): (Sender<ParseResult<Vec<SmtpInput>>>,
                               Receiver<ParseResult<Vec<SmtpInput>>>) = channel();
        (Self { input: rx_inp }, tx_inp)
    }
}

impl SmtpSessionParser for MockParser {
    fn session<'a>(&self, _input: &'a str) -> ParseResult<Vec<SmtpInput>> {
        match self.input.recv() {
            Ok(result) => result,
            _ => panic!("ooh"),
        }
    }
}

pub struct MockWriter;

impl SmtpAnswerWriter for MockWriter {
    fn write(&self, _buf: &mut io::Write, _answer: SmtpReply) -> Result<(), io::Error> {
        Ok(())
    }
}
