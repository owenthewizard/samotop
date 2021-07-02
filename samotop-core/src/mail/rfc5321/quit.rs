use super::Esmtp;
use crate::{
    common::S1Fut,
    smtp::{command::SmtpQuit, Action, SmtpState},
};

impl Action<SmtpQuit> for Esmtp {
    fn apply<'a, 's, 'f>(&'a self, _cmd: SmtpQuit, state: &'s mut SmtpState) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
            let name = state.session.service_name.clone();
            state.reset();
            state.say_shutdown_ok(name);
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

            Esmtp.apply(SmtpQuit, &mut set).await;
            assert!(set.transaction.is_empty())
        })
    }
}
