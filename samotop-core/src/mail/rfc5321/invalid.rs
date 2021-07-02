use super::Esmtp;
use crate::{
    common::S1Fut,
    smtp::{command::SmtpInvalidCommand, Action, SmtpState},
};

impl Action<SmtpInvalidCommand> for Esmtp {
    fn apply<'a, 's, 'f>(
        &'a self,
        _cmd: SmtpInvalidCommand,
        state: &'s mut SmtpState,
    ) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move { state.say_invalid_syntax() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        mail::{Builder, Recipient},
        smtp::{command::SmtpMail, DriverControl, SmtpPath},
    };

    #[test]
    fn response_is_not_implemented() {
        async_std::task::block_on(async move {
            let mut set = SmtpState::new(Builder::default().into_service());
            set.transaction.id = "someid".to_owned();
            set.transaction.mail = Some(SmtpMail::Mail(SmtpPath::Null, vec![]));
            set.transaction.rcpts.push(Recipient::null());
            set.transaction.extra_headers.insert_str(0, "feeeha");

            Esmtp
                .apply(SmtpInvalidCommand::new(b"HOOO".to_vec()), &mut set)
                .await;
            match set.writes.pop_front() {
                Some(DriverControl::Response(bytes)) if bytes.starts_with(b"500 ") => {}
                otherwise => panic!("Expected syntax failure, got {:?}", otherwise),
            }
        })
    }
}
