use crate::common::*;
use crate::smtp::{extension, SmtpSessionCommand, SmtpState};

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct StartTls;

impl SmtpSessionCommand for StartTls {
    fn verb(&self) -> &str {
        "STARTTLS"
    }
    fn apply<'s, 'f, S>(self, mut state: S) -> S2Fut<'f, S>
    where
        S: SmtpState + 's,
        's: 'f,
    {
        if state.session().smtp_helo.is_none() {
            state.say_command_sequence_fail()
        } else {
            // you cannot STARTTLS twice so we only advertise it before first use
            if state.extensions_mut().disable(&extension::STARTTLS) {
                state.reset();
                let name = state.session().service_name.clone();
                state.say_start_tls(name)
            } else {
                state.say_not_implemented()
            }
        }

        Box::pin(ready(state))
    }
}
