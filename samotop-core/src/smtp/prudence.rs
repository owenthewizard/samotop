use crate::common::*;
use crate::mail::{EsmtpService, MailSetup};
use async_std::prelude::FutureExt;
use std::time::Duration;

/// Prevent or monitor bad SMTP behavior
#[derive(Debug)]
pub struct Prudence {
    /// Refuse mail from clients sending commands before banner or just report it in mail headers?
    pub enforce_rfc_wait_for_banner: bool,
    /// Monitor bad behavior of clients not waiting for a banner
    pub check_rfc_wait_for_banner: bool,
}

impl MailSetup for Prudence {
    fn setup(self, config: &mut crate::mail::Configuration) {
        config.esmtp.insert(0, Box::new(self));
    }
}

impl EsmtpService for Prudence {
    fn prepare_session<'a, 'i, 's, 'f>(
        &'a self,
        io: &'i mut dyn crate::io::tls::MayBeTls,
        session: &'s mut crate::mail::SessionInfo,
    ) -> crate::common::S1Fut<'f, ()>
    where
        'a: 'f,
        'i: 'f,
        's: 'f,
    {
        Box::pin(async move {
            if !session.banner_sent
                && (self.check_rfc_wait_for_banner || self.enforce_rfc_wait_for_banner)
            {
                let mut buf = [0u8];
                match io
                        .read(&mut buf[..])
                        .timeout(Duration::from_millis(3210))
                        .await
                        .ok(/*convert timeout result to option*/)
                {
                    Some(Ok(0)) => {
                        // this just looks like the client gave up and left
                    }
                    Some(Ok(_)) => {
                        if self.enforce_rfc_wait_for_banner {
                            todo!("stop now")
                        } else {
                            todo!("add report header")
                        }
                    }
                    Some(Err(_)) => todo!("IO error"),
                    None => {
                        // timeout is correct behavior, well done!
                    }
                }
            }
        })
    }
}
