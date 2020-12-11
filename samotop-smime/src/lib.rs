mod smime;

pub use crate::smime::*;
use samotop_core::{common::*, mail::*};

#[derive(Debug, Clone, Copy)]
pub struct SMimeMail;

impl MailSetup for SMimeMail {
    fn setup(self, builder: &mut Builder) {
        builder.dispatch.insert(0, Box::new(self))
    }
}

impl MailDispatch for SMimeMail {
    fn send_mail<'a, 's, 'f>(
        &'a self,
        session: &'s SessionInfo,
        transaction: Transaction,
    ) -> S2Fut<'f, DispatchResult>
    where
        'a: 'f,
        's: 'f,
    {
        todo!()
    }
}
