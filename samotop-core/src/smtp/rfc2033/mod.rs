mod body;
mod helo;

use crate::{
    common::*,
    io::tls::MayBeTls,
    mail::{AcceptsInterpretter, AcceptsSessionService, MailSetup},
    smtp::{
        command::{MailBody, SmtpCommand},
        *,
    },
};

/// An implementation of LMTP - RFC 2033 - Local Mail Transfer Protocol
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct Lmtp;

pub type Rfc2033 = Lmtp;

impl Lmtp {
    pub fn with<P>(&self, parser: P) -> LmtpConfigured<P>
    where
        P: Parser<SmtpCommand>,
        P: Parser<MailBody<Vec<u8>>>,
        P: Send + Sync + 'static,
    {
        LmtpConfigured {
            parser: Arc::new(parser),
        }
    }
}

#[derive(Debug)]
pub struct LmtpConfigured<P> {
    parser: Arc<P>,
}

impl<T: AcceptsSessionService + AcceptsInterpretter, P> MailSetup<T> for LmtpConfigured<P>
where
    P: Parser<SmtpCommand>,
    P: Parser<MailBody<Vec<u8>>>,
    P: fmt::Debug + Send + Sync + 'static,
{
    fn setup(self, config: &mut T) {
        config.add_last_interpretter(
            Interpretter::default()
                .parse::<SmtpCommand>()
                .with(self.parser.clone())
                .and_apply(Lmtp)
                .parse::<MailBody<Vec<u8>>>()
                .with(self.parser.clone())
                .and_apply(Lmtp),
        );
        config.add_last_session_service(self);
    }
}

impl<P> SessionService for LmtpConfigured<P>
where
    P: Parser<SmtpCommand>,
    P: Parser<MailBody<Vec<u8>>>,
    P: fmt::Debug + Send + Sync + 'static,
{
    fn prepare_session<'a, 'i, 's, 'f>(
        &'a self,
        _io: &'i mut Box<dyn MayBeTls>,
        state: &'s mut SmtpContext,
    ) -> S1Fut<'f, ()>
    where
        'a: 'f,
        'i: 'f,
        's: 'f,
    {
        state.session.say_service_ready();
        Box::pin(ready(()))
    }
}
