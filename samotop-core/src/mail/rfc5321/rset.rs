use crate::common::S1Fut;
use crate::mail::Esmtp;
use crate::smtp::{command::SmtpRset, Action, SmtpState};

impl Action<SmtpRset> for Esmtp {
    fn apply<'a, 's, 'f>(&'a self, _cmd: SmtpRset, state: &'s mut SmtpState) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
            state.reset();
            state.say_ok();
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::{Builder, Recipient},
        smtp::{command::SmtpMail, SmtpPath},
    };

    #[test]
    fn transaction_gets_reset() {
        async_std::task::block_on(async move {
            let mut set = SmtpState::new(Builder::default().into_service());
            set.transaction.id = "someid".to_owned();
            set.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
            set.transaction.rcpts.push(Recipient::null());
            set.transaction.extra_headers.insert_str(0, "feeeha");

            Esmtp.apply(SmtpRset, &mut set).await;
            assert!(set.transaction.is_empty())
        })
    }
}
