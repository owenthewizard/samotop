use crate::{
    common::*,
    io::{
        tls::{Io, MayBeTls, TlsCapable},
        ConnectionInfo, IoService,
    },
    mail::{MailService, SessionInfo},
    smtp::*,
};
use futures::StreamExt;
use smol_timeout::TimeoutExt;
use std::time::{Duration, Instant};

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
/// and serializing `CodecControl` items.
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
            let mut sess = SessionInfo::new(connection, "".to_owned());
            sess.command_timeout = Duration::from_secs(10);

            // Add tls if needed and available
            if !io.can_encrypt() && !io.is_encrypted() {
                if let Some(upgrade) = mail_service.get_tls_upgrade() {
                    let plain: Box<dyn Io> = Box::new(io);
                    io = Box::new(TlsCapable::enabled(plain, upgrade, String::default()));
                }
            }

            // enable STARTTLS extension if it can be used
            if io.can_encrypt() && !io.is_encrypted() {
                sess.extensions.enable(&extension::STARTTLS);
            }

            let parser = mail_service.get_parser_for_commands();
            let mut state = SmtpState::new(mail_service);
            let mut codec = SmtpCodec::new(io);
            codec.send(CodecControl::Parser(parser));

            // send connection info
            state = sess.apply(state).await;
            let mut last = Instant::now();
            loop {
                // fetch and apply commands
                match codec.next().timeout(Duration::from_secs(1)).await {
                    Some(Some(command)) => {
                        state.session.last_command_at = Some(last);
                        state = command.apply(state).await;
                        last = Instant::now();
                    }
                    None => state = Timeout::new(last).apply(state).await,
                    Some(None) => {
                        // client went silent, we're done!
                        SessionShutdown.apply(state).await;
                        break Ok(());
                    }
                }
                // write all pending responses
                for response in state.writes.drain(..) {
                    codec.send(response);
                }
            }
        })
    }
}
