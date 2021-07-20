use super::StartTls;
use crate::common::S1Fut;
use crate::smtp::{extension, Action, SmtpState};

impl Action<StartTls> for StartTls {
    /// Applies given helo to the state
    /// It asserts the right HELO/EHLO variant
    fn apply<'a, 's, 'f>(&'a self, _cmd: StartTls, state: &'s mut SmtpState) -> S1Fut<'f, ()>
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
                    state.say_start_tls()
                } else {
                    state.say_not_implemented()
                }
            }
        })
    }
}
