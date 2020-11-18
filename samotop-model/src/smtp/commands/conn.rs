use crate::{common::ready, mail::SessionInfo, smtp::SmtpSessionCommand};

impl SmtpSessionCommand for SessionInfo {
    fn verb(&self) -> &str {
        ""
    }

    fn apply<'s, 'f, S>(mut self, mut state: S) -> crate::common::S2Fut<'f, S>
    where
        S: crate::smtp::SmtpState + 's,
        's: 'f,
    {
        state.service().prepare_session(&mut self);
        let name = self.service_name.to_owned();
        state.reset();
        *state.session_mut() = self;
        state.say_service_ready(name);
        Box::pin(ready(state))
    }
}

#[derive(Default, Eq, PartialEq, Debug, Clone)]
pub struct SessionShutdown;

impl SmtpSessionCommand for SessionShutdown {
    fn verb(&self) -> &str {
        ""
    }

    fn apply<'s, 'f, S>(self, mut state: S) -> crate::common::S2Fut<'f, S>
    where
        S: crate::smtp::SmtpState + 's,
        's: 'f,
    {
        state.reset();
        *state.session_mut() = SessionInfo::default();
        Box::pin(ready(state))
    }
}
