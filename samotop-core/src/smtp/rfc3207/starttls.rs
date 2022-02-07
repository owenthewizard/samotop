use super::StartTls;
use crate::common::S2Fut;
use crate::smtp::{Action, ExtensionSet, SmtpContext};

impl Action<StartTls> for StartTls {
    /// Applies given helo to the state
    /// It asserts the right HELO/EHLO variant
    fn apply<'a, 's, 'f>(&'a self, _cmd: StartTls, state: &'s mut SmtpContext) -> S2Fut<'f, ()>
    where
        'a: 'f,
        's: 'f,
    {
        Box::pin(async move {
            if state.session.peer_name.is_none() {
                state.session.say_command_sequence_fail()
            } else {
                // you cannot STARTTLS twice so we only advertise it before first use
                if state.session.extensions.disable(&ExtensionSet::STARTTLS) {
                    state.session.reset();
                    state.session.say_start_tls()
                } else {
                    state.session.say_not_implemented()
                }
            }
        })
    }
}
