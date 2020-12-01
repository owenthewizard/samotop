use samotop_model::io::tls::{Io, TlsCapable};

use crate::common::*;
use crate::{
    io::{tls::MayBeTls, ConnectionInfo, IoService},
    mail::{MailService, SessionInfo},
    protocol::smtp::SmtpCodec,
    smtp::*,
};
use std::sync::Arc;

/// `SmtpService` provides a stateful SMTP service on a TCP, Unix or other asyn IO conection.
///
/// It uses the given `MailService` which takes care of mail events:
/// * session setup - upon a new connection
/// * opening new mail transaction - MAIL FROM:<x@y.z>
/// * adding recipients - RCPT FROM:<a@b.c>
/// * streaming data - DATA...
///
/// It uses the given `Parser` to extract SMTP commands from the input strings.
/// This is essential because the commands drive the session. All commands
/// are `apply()`d to the `SmtpState`.
///
/// Internally it uses the `SmtpCodec` responsible for extracting `ReadControl`
/// and serializing `WriteControl` items.
///
/// It is effectively a composition and setup of components required to serve SMTP.
///
#[derive(Clone)]
pub struct SmtpService<S> {
    mail_service: Arc<S>,
}

impl<S> SmtpService<S>
where
    S: MailService + Send + Sync + 'static,
{
    pub fn new(mail_service: S) -> Self {
        Self {
            mail_service: Arc::new(mail_service),
        }
    }
}

impl<S> IoService for SmtpService<S>
where
    S: MailService + Send + Sync + 'static,
{
    fn handle(
        &self,
        io: Result<Box<dyn MayBeTls>>,
        connection: ConnectionInfo,
    ) -> S3Fut<Result<()>> {
        let mail_service = self.mail_service.clone();

        Box::pin(async move {
            info!("New peer connection {}", connection);

            let mut io = io?;

            if !io.can_encrypt() && !io.is_encrypted() {
                if let Some(upgrade) = mail_service.get() {
                    let plain: Box<dyn Io> = Box::new(io);
                    io = Box::new(TlsCapable::enabled(plain, upgrade, String::default()));
                }
            }

            let mut sess = SessionInfo::new(connection, "".to_owned());

            if io.can_encrypt() && !io.is_encrypted() {
                sess.extensions.enable(&extension::STARTTLS);
            }
            let mut state = SmtpState::new(mail_service);
            let mut codec = SmtpCodec::new(io);

            // send connection info
            state = sess.apply(state).await;

            loop {
                // write all pending responses
                for response in state.writes.drain(..) {
                    codec.send(response).await?;
                }
                // fetch and apply commands
                if let Some(command) = codec.next().await {
                    state = command.apply(state).await;
                } else {
                    // client went silent, we're done!
                    SessionShutdown.apply(state).await;
                    return Ok(());
                }
            }
        })
    }
}
