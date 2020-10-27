//! SMTP internal utilities

mod codec;
//mod mock;
mod proto;
mod xtext;

pub use codec::*;
// CHECKME: Was this part of the API?
//pub use mock::*;
pub use proto::*;
pub use xtext::*;
