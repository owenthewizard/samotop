use crate::{
    common::*,
    parser::Parser,
    smtp::{CodecControl, MailBodyChunk, MailBodyEnd, SmtpSessionCommand, SmtpState},
};

use super::Rfc2033;

impl SmtpSessionCommand for Rfc2033<MailBodyEnd> {
    fn verb(&self) -> &str {
        ""
    }

    fn apply(&self, mut state: SmtpState) -> S2Fut<SmtpState> {
        if state.transaction.sink.is_none() {
            // CheckMe: silence. handle_data_end should respond with error.
            return Box::pin(ready(state));
        }
        let mut sink = state
            .transaction
            .sink
            .take()
            .expect("Checked presence above");
        let mailid = state.transaction.id.clone();
        let fut = async move {
            if match poll_fn(move |cx| sink.as_mut().poll_close(cx)).await {
                Ok(()) => true,
                Err(e) if e.kind() == std::io::ErrorKind::NotConnected => true,
                Err(e) => {
                    warn!("Failed to close mail {}: {}", mailid, e);
                    false
                }
            } {
                for msg in state
                    .transaction
                    .rcpts
                    .iter()
                    .map(|rcpt| format!("{} for {}", mailid, rcpt.address))
                    .collect::<Vec<String>>()
                {
                    state.say_mail_queued(msg.as_str());
                }
            } else {
                state.say_mail_queue_failed_temporarily();
            }
            state.reset();
            state.say(CodecControl::Parser(
                state.service.get_parser_for_commands(),
            ));
            state
        };
        Box::pin(fut)
    }
}
