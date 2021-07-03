use crate::{
    common::*,
    io::{
        tls::{Io, MayBeTls, TlsCapable},
        ConnectionInfo, IoService,
    },
    mail::{Esmtp, MailService, SessionInfo},
    smtp::*,
};

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
/// Internally it uses the `SmtpDriver` responsible for parsing commands and applying `Action`s
/// and serializing `DriverControl` items.
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
    ) -> S1Fut<'static, Result<()>> {
        let mail_service = self.mail_service.clone();

        Box::pin(async move {
            info!("New peer connection {}", connection);

            let mut io = io?;
            let mut sess = SessionInfo::new(connection, "".to_owned());

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

            let mut driver = SmtpDriver::new(io);

            let interpretter = mail_service.get_interpretter();
            let mut state = SmtpState::new(mail_service);

            // send connection info
            Esmtp.apply(sess, &mut state).await;

            while driver.is_open() {
                // fetch and apply commands
                driver.drive(&interpretter, &mut state).await?
            }

            Ok(())
        })
    }
}
