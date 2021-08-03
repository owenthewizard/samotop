use super::Esmtp;
use crate::{
    common::S1Fut,
    smtp::{command::SmtpInvalidCommand, Action, SmtpContext},
};

impl Action<SmtpInvalidCommand> for Esmtp {
    fn apply<'a, 's, 'f>(
        &'a self,
        _cmd: SmtpInvalidCommand,
        state: &'s mut SmtpContext,
    ) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move { state.session.say_invalid_syntax() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::Recipient,
        smtp::{command::SmtpMail, DriverControl, SmtpPath},
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
                .apply(SmtpInvalidCommand::new(b"HOOO".to_vec()), &mut set)
                .await;
            match set.session.pop_control() {
                Some(DriverControl::Response(bytes)) if bytes.starts_with(b"500 ") => {}
                otherwise => panic!("Expected syntax failure, got {:?}", otherwise),
            }
        })
    }
}
