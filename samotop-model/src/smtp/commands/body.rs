use crate::{
    common::*,
    smtp::{SmtpSessionCommand, SmtpState},
};

/// A chunk of the mail body
#[derive(Default, Eq, PartialEq, Debug, Clone)]
pub struct MailBodyChunk(pub Vec<u8>);

/// The mail body is finished. Mail should be queued.
#[derive(Default, Eq, PartialEq, Debug, Clone)]
pub struct MailBodyEnd;

impl SmtpSessionCommand for MailBodyChunk {
    fn verb(&self) -> &str {
        ""
    }

    fn apply(self, mut state: SmtpState) -> S3Fut<SmtpState> {
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
            let write_all = WriteAll {
                from: &self.0[..],
                to: &mut sink,
            };
            match write_all.await {
                Ok(()) => {
                    state.transaction.sink = Some(sink);
                    state
                }
                Err(e) => {
                    warn!("Failed to write mail data for {} - {}", mailid, e);
                    state.reset();
                    // CheckMe: following this reset, we are not sending any response yet. MailBodyEnd should do that.
                    state
                }
            }
        };
        Box::pin(fut)
    }
}
impl SmtpSessionCommand for MailBodyEnd {
    fn verb(&self) -> &str {
        ""
    }

    fn apply(self, mut state: SmtpState) -> S3Fut<SmtpState> {
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
            state
        };
        Box::pin(fut)
    }
}

struct WriteAll<'a, W> {
    pub from: &'a [u8],
    pub to: W,
}

impl<W> Future for WriteAll<'_, W>
where
    W: Write + Unpin,
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
