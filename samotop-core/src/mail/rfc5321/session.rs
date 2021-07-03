use crate::{
    common::S1Fut,
    mail::{Esmtp, SessionInfo},
    smtp::{
        command::{ProcessingError, SessionShutdown, Timeout},
        Action, SmtpReply, SmtpState,
    },
};

impl Action<SessionInfo> for Esmtp {
    fn apply<'a, 's, 'f>(&'a self, cmd: SessionInfo, state: &'s mut SmtpState) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
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
        })
    }
}

impl Action<SessionShutdown> for Esmtp {
    fn apply<'a, 's, 'f>(&'a self, _cmd: SessionShutdown, state: &'s mut SmtpState) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
            state.shutdown();
        })
    }
}

impl Action<Timeout> for Esmtp {
    fn apply<'a, 's, 'f>(&'a self, _cmd: Timeout, state: &'s mut SmtpState) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
            state.say_shutdown_service_err("Timeout expired.".to_owned());
        })
    }
}

impl Action<ProcessingError> for Esmtp {
    fn apply<'a, 's, 'f>(&'a self, _cmd: ProcessingError, state: &'s mut SmtpState) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
            state.say_shutdown(SmtpReply::ProcesingError);
        })
    }
}
