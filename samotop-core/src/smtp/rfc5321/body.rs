use super::Esmtp;
use crate::{
    common::*,
    smtp::{command::MailBody, Action, SmtpContext, SmtpSession},
};

impl<B: AsRef<[u8]> + Sync + Send + fmt::Debug + 'static> Action<MailBody<B>> for Esmtp {
    fn apply<'a, 's, 'f>(&'a self, cmd: MailBody<B>, state: &'s mut SmtpContext) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(apply_mail_body(false, cmd, state))
    }
}

pub async fn apply_mail_body<B>(lmtp: bool, cmd: MailBody<B>, state: &mut SmtpContext)
where
    B: AsRef<[u8]> + Sync + Send + fmt::Debug + 'static,
{
    let sink = state.session.transaction.sink.take();
    let mailid = state.session.transaction.id.clone();

    match cmd {
        MailBody::Chunk {
            data,
            ends_with_new_line,
        } => {
            let mut sink = if let Some(sink) = sink {
                sink
            } else {
                // CheckMe: silence. MailBody::End should respond with error.
                return;
            };

            let mut copy_from = data.as_ref();
            let mut copy_to = sink.as_mut();
            let copy = Box::pin(poll_fn(move |cx| loop {
                match copy_to.as_mut().poll_write(cx, copy_from)? {
                    Poll::Ready(written) => copy_from = &copy_from[written..],
                    Poll::Pending => return Poll::Pending,
                }
                if copy_from.is_empty() {
                    break Poll::Ready(Ok::<(), io::Error>(()));
                }
            }));

            match copy.await {
                Ok(()) => {
                    //let WriteAll { to, .. } = write_all;
                    state.session.transaction.sink = Some(sink);
                    state.session.mode = Some(match ends_with_new_line {
                        true => SmtpSession::DATA_MODE,
                        false => SmtpSession::DATA_PARTIAL_MODE,
                    })
                }
                Err(e) => {
                    warn!("Failed to write mail data for {} - {}", mailid, e);
                    state.session.reset();
                    // CheckMe: following this reset, we are not sending any response yet. MailBodyEnd should do that.
                }
            };
        }
        MailBody::End => {
            let mut sink = if let Some(sink) = sink {
                sink
            } else {
                state.session.say_mail_queue_failed_temporarily();
                state.session.reset();
                return;
            };
            if match poll_fn(move |cx| sink.as_mut().poll_close(cx)).await {
                Ok(()) => true,
                Err(e) if e.kind() == std::io::ErrorKind::NotConnected => true,
                Err(e) => {
                    warn!("Failed to close mail {}: {}", mailid, e);
                    false
                }
            } {
                if lmtp {
                    for msg in state
                        .session
                        .transaction
                        .rcpts
                        .iter()
                        .map(|rcpt| format!("{} for {}", mailid, rcpt.address))
                        .collect::<Vec<String>>()
                    {
                        state.session.say_mail_queued(msg.as_str());
                    }
                } else {
                    state.session.say_mail_queued(mailid.as_str());
                }
            } else {
                state.session.say_mail_queue_failed_temporarily();
            }
            state.session.reset();
        }
    }
}
