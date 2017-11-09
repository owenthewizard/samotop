use std::io;
use std::io::Write;
use model::response::SmtpReply;

static SERIALIZER: SmtpWriter = SmtpWriter;

type Result = io::Result<()>;

pub trait SmtpAnswerWriter {
    fn write(&self, buf: &mut Write, answer: SmtpReply) -> Result;
}

pub struct SmtpWriter;

impl SmtpWriter {
    pub fn answer_writer<'a>() -> &'a SmtpAnswerWriter {
        &SERIALIZER
    }
}

impl SmtpAnswerWriter for SmtpWriter {
    fn write(&self, mut buf: &mut Write, reply: SmtpReply) -> Result {
        // take default display implementation
        write!(buf, "{}", reply)
    }
}
