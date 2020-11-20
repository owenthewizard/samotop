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

        if state.session().service_name.is_empty() {
            if let Some(addr) = state.session().connection.local_addr {
                state.session_mut().service_name = addr.to_string();
                debug!(
                    "Service name is empty. Using local address instead {:?}",
                    state.session().service_name
                );
            } else {
                state.session_mut().service_name = "samotop".to_owned();
                debug!(
                    "Service name is empty. Using default {:?}",
                    state.session().service_name
                );
            }
        } else {
            debug!("Service name is {:?}", state.session().service_name);
        }

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
