use crate::{
    mail::{Esmtp, SessionInfo},
    smtp::{
        command::{ProcessingError, SessionShutdown, Timeout},
        Action, SmtpReply, SmtpState,
    },
};

#[async_trait::async_trait]
impl Action<SessionInfo> for Esmtp {
    async fn apply(&self, cmd: SessionInfo, state: &mut SmtpState) {
        state.session = cmd;
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
            info!("Service name is {:?}", state.session.service_name);
        }

        let name = state.session.service_name.to_owned();
        state.reset();
        state.say_service_ready(name);
    }
}

#[async_trait::async_trait]
impl Action<SessionShutdown> for Esmtp {
    async fn apply(&self, _cmd: SessionShutdown, state: &mut SmtpState) {
        state.shutdown();
    }
}

#[async_trait::async_trait]
impl Action<Timeout> for Esmtp {
    async fn apply(&self, _cmd: Timeout, state: &mut SmtpState) {
        state.say_shutdown_service_err("Timeout expired.".to_owned());
    }
}

#[async_trait::async_trait]
impl Action<ProcessingError> for Esmtp {
    async fn apply(&self, _cmd: ProcessingError, state: &mut SmtpState) {
        state.say_shutdown(SmtpReply::ProcesingError);
    }
}
