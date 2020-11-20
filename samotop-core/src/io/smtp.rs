use crate::common::*;
use crate::{
    io::{ConnectionInfo, IoService, MayBeTls},
    mail::{MailService, SessionInfo},
    parser::Parser,
    protocol::{parse::*, smtp::SmtpCodec},
    smtp::*,
};
use futures::SinkExt;
use std::marker::PhantomData;
use std::sync::Arc;

/// `SmtpService` provides an SMTP service on a TCP conection
/// using the given `SessionService` which:
/// * handles `ReadControl`s, such as SMTP commands and connection events,
/// * drives the session state, and
/// * produces `WriteControl`s.
///
/// Behind the scenes it uses the `SmtpParser` to extract SMTP commands from the input strings.
///
/// It is effectively a composition and setup of components required to serve SMTP.
///
#[derive(Clone)]
pub struct SmtpService<S, P, IO> {
    mail_service: Arc<S>,
    parser: Arc<P>,
    phantom: PhantomData<IO>,
}

impl<S, P, IO> SmtpService<S, P, IO>
where
    S: MailService + Send + Sync + 'static,
    P: Parser + Sync + Send + 'static,
    IO: MayBeTls + Read + Write + Unpin + Sync + Send + 'static,
{
    pub fn new(mail_service: S, parser: P) -> Self {
        Self {
            mail_service: Arc::new(mail_service),
            parser: Arc::new(parser),
            phantom: PhantomData,
        }
    }
}

impl<S, P, IO> IoService<IO> for SmtpService<S, P, IO>
where
    S: MailService + Send + Sync + 'static,
    IO: MayBeTls + Read + Write + Unpin + Sync + Send + 'static,
    P: Parser + Sync + Send + 'static,
{
    fn handle(&self, io: Result<IO>, connection: ConnectionInfo) -> S3Fut<Result<()>> {
        let mail_service = self.mail_service.clone();
        let parser = self.parser.clone();

        Box::pin(async move {
            info!("New peer connection {}", connection);
            let io = io?;
            let mut sess = SessionInfo::new(connection, "".to_owned());
            if io.can_encrypt() && !io.is_encrypted() {
                sess.extensions.enable(&extension::STARTTLS);
            }
            let state = SmtpState::new(mail_service);
            let codec = SmtpCodec::new(io, sess);
            let sink = codec.get_sender();
            let stream = codec.parse(parser);
            let stream = SessionStream::new(stream, state);
            stream.forward(sink.sink_err_into()).await
        })
    }
}
