use crate::common::*;
use crate::mail::{EsmtpStartTls, Rfc3207};
use crate::smtp::{extension, ApplyCommand, SmtpSessionCommand, SmtpState};

impl SmtpSessionCommand for EsmtpStartTls {
    fn verb(&self) -> &str {
        "STARTTLS"
    }

    fn apply(&self, state: SmtpState) -> S1Fut<SmtpState> {
        EsmtpStartTls::apply_cmd(&self, state)
    }
}

impl ApplyCommand<EsmtpStartTls> for Rfc3207 {
    /// Applies given helo to the state
    /// It asserts the right HELO/EHLO variant
    fn apply_cmd(_cmd: &EsmtpStartTls, mut state: SmtpState) -> S1Fut<SmtpState> {
        if state.session.peer_name.is_none() {
            state.say_command_sequence_fail()
        } else {
            // you cannot STARTTLS twice so we only advertise it before first use
            if state.session.extensions.disable(&extension::STARTTLS) {
                state.reset();
                let name = state.session.service_name.clone();
                state.say_start_tls(name)
            } else {
                state.say_not_implemented()
            }
        }

        Box::pin(ready(state))
    }
}
