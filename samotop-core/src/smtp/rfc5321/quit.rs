use super::Esmtp;
use crate::{
    common::S1Fut,
    smtp::{command::SmtpQuit, Action, SmtpContext},
};

impl Action<SmtpQuit> for Esmtp {
    fn apply<'a, 's, 'f>(&'a self, _cmd: SmtpQuit, state: &'s mut SmtpContext) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
            state.session.reset();
            state.session.say_shutdown_ok();
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::Recipient,
        smtp::{command::SmtpMail, SmtpPath},
    };

    #[test]
    fn transaction_gets_reset() {
        async_std::task::block_on(async move {
            let mut set = SmtpContext::default();
            set.session.transaction.id = "someid".to_owned();
            set.session.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
            set.session.transaction.rcpts.push(Recipient::null());
            set.session
                .transaction
                .extra_headers
                .insert_str(0, "feeeha");

            Esmtp.apply(SmtpQuit, &mut set).await;
            assert!(set.session.transaction.is_empty())
        })
    }
}
