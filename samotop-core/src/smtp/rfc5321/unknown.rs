use super::Esmtp;
use crate::{
    common::S1Fut,
    smtp::{command::SmtpUnknownCommand, Action, SmtpContext},
};

impl Action<SmtpUnknownCommand> for Esmtp {
    fn apply<'a, 's, 'f>(
        &'a self,
        _cmd: SmtpUnknownCommand,
        state: &'s mut SmtpContext,
    ) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
            state.session.say_not_implemented();
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::Recipient,
        smtp::{command::SmtpMail, DriverControl, SmtpContext, SmtpPath},
    };

    #[test]
    fn response_is_not_implemented() {
        async_std::task::block_on(async move {
            let mut set = SmtpContext::default();
            set.session.transaction.id = "someid".to_owned();
            set.session.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
            set.session.transaction.rcpts.push(Recipient::null());
            set.session
                .transaction
                .extra_headers
                .insert_str(0, "feeeha");

            Esmtp
                .apply(SmtpUnknownCommand::new("HOOO".to_owned(), vec![]), &mut set)
                .await;
            match set.session.pop_control() {
                Some(DriverControl::Response(bytes)) if bytes.starts_with(b"502 ") => {}
                otherwise => panic!("Expected command not implemented, got {:?}", otherwise),
            }
        })
    }
}
