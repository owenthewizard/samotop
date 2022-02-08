use crate::{
    common::S2Fut,
    smtp::{command::SmtpRset, Action, Esmtp, SmtpContext},
};

impl Action<SmtpRset> for Esmtp {
    fn apply<'a, 's, 'f>(&'a self, _cmd: SmtpRset, state: &'s mut SmtpContext) -> S2Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
            state.session.reset();
            state.session.say_ok();
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::Recipient,
        smtp::{command::SmtpMail, SmtpPath, SmtpSession},
        config::Store,
    };

    #[test]
    fn transaction_gets_reset() {
        async_std::task::block_on(async move {
            let mut store = Store::default();
            let mut smtp = SmtpSession::default();
            let mut set = SmtpContext::new(&mut store, &mut smtp);

            set.session.transaction.id = "someid".to_owned();
            set.session.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
            set.session.transaction.rcpts.push(Recipient::null());
            set.session
                .transaction
                .extra_headers
                .insert_str(0, "feeeha");

            Esmtp.apply(SmtpRset, &mut set).await;
            assert!(set.session.transaction.is_empty())
        })
    }
}
