mod builder;
mod debug;
mod dispatch;
mod esmtp;
mod guard;
mod mailservice;
mod name;
mod null;
mod parser;
mod recipient;
mod rfc2033;
mod rfc3207;
mod rfc5321;
mod rfc821;
mod session;
mod setup;
mod tls;
mod transaction;

pub use self::builder::*;
pub use self::debug::*;
pub use self::dispatch::*;
pub use self::esmtp::*;
pub use self::guard::*;
pub use self::mailservice::*;
pub use self::name::*;
pub use self::null::*;
pub use self::parser::*;
pub use self::recipient::*;
pub use self::rfc2033::*;
pub use self::rfc3207::*;
pub use self::rfc5321::*;
pub use self::rfc5321::*;
pub use self::rfc821::*;
pub use self::session::*;
pub use self::setup::*;
pub use self::tls::*;
pub use self::transaction::*;

use crate::{
    common::{ready, S2Fut},
    smtp::{SmtpHelo, SmtpState},
};

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
