use super::Esmtp;
use crate::{
    common::*,
    mail::Transaction,
    smtp::{command::MailBody, Action, SmtpState},
};

impl<B: AsRef<[u8]> + Sync + Send + fmt::Debug + 'static> Action<MailBody<B>> for Esmtp {
    fn apply<'a, 's, 'f>(&'a self, cmd: MailBody<B>, state: &'s mut SmtpState) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(apply_mail_body(false, cmd, state))
    }
}

pub async fn apply_mail_body<B>(lmtp: bool, cmd: MailBody<B>, state: &mut SmtpState)
where
    B: AsRef<[u8]> + Sync + Send + fmt::Debug + 'static,
{
    let sink = state.transaction.sink.take();
    let mailid = state.transaction.id.clone();

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
            let mut write_all = sink.write_all(data.as_ref());

            match (&mut write_all).await {
                Ok(()) => {
                    //let WriteAll { to, .. } = write_all;
                    state.transaction.sink = Some(sink);
                    state.transaction.mode = Some(match ends_with_new_line {
                        true => Transaction::DATA_MODE,
                        false => Transaction::DATA_PARTIAL_MODE,
                    })
                }
                Err(e) => {
                    warn!("Failed to write mail data for {} - {}", mailid, e);
                    state.reset();
                    // CheckMe: following this reset, we are not sending any response yet. MailBodyEnd should do that.
                }
            };
        }
        MailBody::End => {
            let mut sink = if let Some(sink) = sink {
                sink
            } else {
                state.say_mail_queue_failed_temporarily();
                state.reset();
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
                        .transaction
                        .rcpts
                        .iter()
                        .map(|rcpt| format!("{} for {}", mailid, rcpt.address))
                        .collect::<Vec<String>>()
                    {
                        state.say_mail_queued(msg.as_str());
                    }
                } else {
                    state.say_mail_queued(mailid.as_str());
                }
            } else {
                state.say_mail_queue_failed_temporarily();
            }
            state.reset();
        }
    }
}
