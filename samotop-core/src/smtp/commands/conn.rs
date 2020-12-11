use crate::{
    common::*,
    mail::SessionInfo,
    smtp::{SmtpReply, SmtpSessionCommand, SmtpState},
};
use std::time::{Duration, Instant};

impl SmtpSessionCommand for SessionInfo {
    fn verb(&self) -> &str {
        ""
    }

    fn apply(&self, mut state: SmtpState) -> S2Fut<SmtpState> {
        state.session = self.clone();
        state.service.prepare_session(&mut state.session);

        if state.session.service_name.is_empty() {
            if !state.session.connection.local_addr.is_empty() {
                state.session.service_name = state.session.connection.local_addr.clone();
                warn!(
                    "Service name is empty. Using local address instead {:?}",
                    state.session.service_name
                );
            } else {
                state.session.service_name = "samotop".to_owned();
                warn!(
                    "Service name is empty. Using default {:?}",
                    state.session.service_name
                );
            }
        } else {
            debug!("Service name is {:?}", state.session.service_name);
        }

        let name = state.session.service_name.to_owned();
        state.reset();
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

    fn apply(&self, mut state: SmtpState) -> S2Fut<SmtpState> {
        state.reset();
        state.session = SessionInfo::default();
        Box::pin(ready(state))
    }
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct Timeout {
    last: Instant,
}
impl Timeout {
    pub fn new(last: Instant) -> Self {
        Self { last }
    }
}
impl SmtpSessionCommand for Timeout {
    fn verb(&self) -> &str {
        ""
    }

    fn apply(&self, mut state: SmtpState) -> S2Fut<SmtpState> {
        let timeout = state.session.command_timeout;
        if timeout > Duration::default() && self.last.elapsed() > timeout {
            state.say_shutdown_err("Timeout expired.".to_owned());
        }

        Box::pin(ready(state))
    }
}

#[derive(Default, Eq, PartialEq, Debug, Clone)]
pub struct ProcessingError;

impl SmtpSessionCommand for ProcessingError {
    fn verb(&self) -> &str {
        ""
    }

    fn apply(&self, mut state: SmtpState) -> S2Fut<SmtpState> {
        state.say_shutdown(SmtpReply::ProcesingError);
        Box::pin(ready(state))
    }
}
