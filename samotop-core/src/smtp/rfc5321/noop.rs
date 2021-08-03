use super::Esmtp;
use crate::{
    common::S1Fut,
    smtp::{command::SmtpNoop, Action, SmtpContext},
};

impl Action<SmtpNoop> for Esmtp {
    fn apply<'a, 's, 'f>(&'a self, _cmd: SmtpNoop, state: &'s mut SmtpContext) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move { state.session.say_ok() })
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

            Esmtp.apply(SmtpNoop, &mut set).await;
            // TODO: assert
        })
    }
}
