use super::Rfc3207;
use crate::common::*;
use crate::smtp::{extension, SmtpSessionCommand, SmtpState, StartTls};

impl SmtpSessionCommand for Rfc3207<StartTls> {
    fn verb(&self) -> &str {
        "STARTTLS"
    }

    fn apply(&self, mut state: SmtpState) -> S3Fut<SmtpState> {
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
