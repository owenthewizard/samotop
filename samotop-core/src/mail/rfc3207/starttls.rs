use crate::common::S1Fut;
use crate::mail::EsmtpStartTls;
use crate::smtp::{extension, Action, SmtpState};

impl Action<EsmtpStartTls> for EsmtpStartTls {
    /// Applies given helo to the state
    /// It asserts the right HELO/EHLO variant
    fn apply<'a, 's, 'f>(&'a self, _cmd: EsmtpStartTls, state: &'s mut SmtpState) -> S1Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
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
        })
    }
}
