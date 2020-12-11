use crate::common::*;
use crate::mail::{ESMTPStartTls, Rfc3207};
use crate::smtp::{extension, ApplyCommand, SmtpSessionCommand, SmtpState};

impl SmtpSessionCommand for ESMTPStartTls {
    fn verb(&self) -> &str {
        "STARTTLS"
    }

    fn apply(&self, state: SmtpState) -> S2Fut<SmtpState> {
        ESMTPStartTls::apply_cmd(&self, state)
    }
}

impl ApplyCommand<ESMTPStartTls> for Rfc3207 {
    /// Applies given helo to the state
    /// It asserts the right HELO/EHLO variant
    fn apply_cmd(_cmd: &ESMTPStartTls, mut state: SmtpState) -> S2Fut<SmtpState> {
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
