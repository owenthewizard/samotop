use crate::{
    common::*,
    smtp::{ApplyCommand, CodecControl, MailBodyChunk, MailBodyEnd, SmtpSessionCommand, SmtpState},
};

use super::{EsmtpCommand, Rfc5321};

impl<B: AsRef<[u8]> + Sync + Send + fmt::Debug> SmtpSessionCommand
    for EsmtpCommand<MailBodyChunk<B>>
{
    fn verb(&self) -> &str {
        ""
    }

    fn apply(&self, state: SmtpState) -> S1Fut<SmtpState> {
        Rfc5321::apply_cmd(&self.instruction, state)
    }
}

impl<B: AsRef<[u8]> + Sync + Send + fmt::Debug> ApplyCommand<MailBodyChunk<B>> for Rfc5321 {
    fn apply_cmd(data: &MailBodyChunk<B>, mut state: SmtpState) -> S1Fut<SmtpState> {
        if state.transaction.sink.is_none() {
            // CheckMe: silence. handle_data_end should respond with error.
            return Box::pin(ready(state));
        }
        let sink = state
            .transaction
            .sink
            .take()
            .expect("Checked presence above");
        let mailid = state.transaction.id.clone();
        let fut = async move {
            let mut write_all = WriteAll {
                from: data.0.as_ref(),
                to: Box::pin(sink),
            };
            match (&mut write_all).await {
                Ok(()) => {
                    let WriteAll { to, .. } = write_all;
                    state.transaction.sink = Some(to);
                }
                Err(e) => {
                    warn!("Failed to write mail data for {} - {}", mailid, e);
                    state.reset();
                    // CheckMe: following this reset, we are not sending any response yet. MailBodyEnd should do that.
                }
            };
            state
        };
        Box::pin(fut)
    }
}

impl SmtpSessionCommand for EsmtpCommand<MailBodyEnd> {
    fn verb(&self) -> &str {
        ""
    }
    fn apply(&self, state: SmtpState) -> S1Fut<SmtpState> {
        Rfc5321::apply_cmd(&self.instruction, state)
    }
}

impl ApplyCommand<MailBodyEnd> for Rfc5321 {
    fn apply_cmd(_data: &MailBodyEnd, mut state: SmtpState) -> S1Fut<SmtpState> {
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
                state.say_mail_queued(mailid.as_str());
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

struct WriteAll<'a, W> {
    pub from: &'a [u8],
    pub to: Pin<Box<W>>,
}

impl<W> Future for WriteAll<'_, W>
where
    W: Write,
{
    type Output = std::io::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        while !this.from.is_empty() {
            let n = match Pin::new(&mut this.to).poll_write(cx, this.from)? {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(len) => len,
            };
            {
                let (_, rest) = std::mem::replace(&mut this.from, &[]).split_at(n);
                this.from = rest;
            }
            if n == 0 {
                return Poll::Ready(Err(std::io::ErrorKind::WriteZero.into()));
            }
        }

        Poll::Ready(Ok(()))
    }
}
