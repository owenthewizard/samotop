use crate::{
    common::S1Fut,
    mail::Esmtp,
    smtp::{
        command::{ProcessingError, SessionSetup, SessionShutdown, Timeout},
        Action, SmtpReply, SmtpState,
    },
};

impl Action<SessionSetup> for Esmtp {
    fn apply<'a, 's, 'f>(&'a self, _cmd: SessionSetup, state: &'s mut SmtpState) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
            state.service.prepare_session(&mut state.session);

            if state.session.service_name.is_empty() {
                state.session.service_name = "samotop".to_owned();
                warn!(
                    "Service name is empty. Using default {:?}",
                    state.session.service_name
                );
            } else {
                info!("Service name is {:?}", state.session.service_name);
            }

            state.reset();
            state.say_service_ready();
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
            state.say_shutdown_timeout();
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
