mod rfc2033;
mod rfc3207;
mod rfc5321;
mod rfc821;

pub use self::rfc2033::*;
pub use self::rfc5321::*;
pub use self::rfc5321::*;
pub use self::rfc821::*;
pub use samotop_model::mail::*;

use crate::common::*;
use crate::smtp::*;

/// Applies given helo to the state
/// It assumes it is the right HELO/EHLO/LHLO variant
fn apply_helo(helo: &SmtpHelo, is_extended: bool, mut state: SmtpState) -> S2Fut<SmtpState> {
    let local = state.session.service_name.to_owned();
    let remote = helo.host.to_string();

    state.reset_helo(helo.host.to_string());

    match is_extended {
        false => state.say_helo(local, remote),
        true => {
            let extensions = state.session.extensions.iter().map(String::from).collect();
            state.say_ehlo(local, extensions, remote)
        }
    };

    Box::pin(ready(state))
}
