use crate::common::*;
use crate::model::io::ConnectionInfo;
use crate::model::mail::SessionInfo;
use crate::model::smtp::extension;
use crate::protocol::parse::*;
use crate::protocol::smtp::SmtpCodec;
use crate::protocol::tls::MayBeTls;
use crate::service::parser::Parser;
use crate::service::session::*;
use crate::service::tcp::TcpService;
use futures::stream::SplitStream;
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
    session_service: Arc<S>,
    parser: Arc<P>,
    phantom: PhantomData<IO>,
}

impl<S, P, IO> SmtpService<S, P, IO>
where
    S: SessionService<SessionInput<IO, Arc<P>>> + Send + Sync + 'static,
    P: Parser + Sync + Send + 'static,
    IO: MayBeTls + Read + Write + Unpin + Sync + Send + 'static,
{
    pub fn new(session_service: S, parser: P) -> Self {
        Self {
            session_service: Arc::new(session_service),
            parser: Arc::new(parser),
            phantom: PhantomData,
        }
    }
}

impl<S, P, IO> TcpService<IO> for SmtpService<S, P, IO>
where
    S: SessionService<SessionInput<IO, Arc<P>>> + Send + Sync + 'static,
    IO: MayBeTls + Read + Write + Unpin + Sync + Send + 'static,
    P: Parser + Sync + Send + 'static,
{
    fn handle(&self, io: Result<IO>, connection: ConnectionInfo) -> S3Fut<Result<()>> {
        let session_service = self.session_service.clone();
        let parser = self.parser.clone();

        Box::pin(async move {
            info!("New peer connection {}", connection);
            let io = io?;
            let mut sess = SessionInfo::new(connection, "".to_owned());
            if io.can_encrypt() && !io.is_encrypted() {
                sess.extensions.enable(&extension::STARTTLS);
            }
            let (dst, src) = SmtpCodec::new(io, sess).split();
            let handler = session_service.start(src.parse(parser)).await;
            handler.forward(dst).await
        })
    }
}

type SessionInput<IO, P> = Parse<SplitStream<SmtpCodec<IO>>, P>;
