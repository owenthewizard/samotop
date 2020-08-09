use crate::common::*;
use crate::grammar::SmtpParser;
use crate::model::io::Connection;
use crate::protocol::*;
use crate::service::session::*;
use crate::service::tcp::TcpService;
use async_std::task;

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
pub struct SmtpService<S> {
    session_service: S,
}

impl<S> SmtpService<S> {
    pub fn new(session_service: S) -> Self {
        Self { session_service }
    }
}

impl<S, IO> TcpService<IO> for SmtpService<S>
where
    S: SessionService + Clone + Send + Sync + 'static,
    S::Handler: Send,
    IO: TlsCapableIO + Read + Write + Unpin + Sync + Send + 'static,
{
    type Future = future::Ready<()>;
    fn handle(self, io: Result<IO>, conn: Connection) -> Self::Future {
        let session_service = self.session_service.clone();
        spawn_task_and_swallow_log_errors(
            format!("SMTP transmission {}", conn),
            handle_smtp(session_service, conn, io),
        );

        future::ready(())
    }
}

fn spawn_task_and_swallow_log_errors<F>(task_name: String, fut: F) -> task::JoinHandle<()>
where
    F: Future<Output = Result<()>> + Send + 'static,
{
    task::spawn(async move {
        log_errors(task_name, fut).await.unwrap();
        ()
    })
}

async fn log_errors<F, T, E>(task_name: String, fut: F) -> F::Output
where
    F: Future<Output = std::result::Result<T, E>>,
    E: std::fmt::Display,
{
    match fut.await {
        Err(e) => {
            error!("Error in {}: {}", task_name, e);
            Err(e)
        }
        Ok(r) => {
            info!("{} completed successfully.", task_name);
            Ok(r)
        }
    }
}

async fn handle_smtp<IO, S>(
    session_service: S,
    connection: Connection,
    io: Result<IO>,
) -> Result<()>
where
    IO: TlsCapableIO + Read + Write + Unpin,
    S: SessionService,
{
    info!("New peer connection {}", connection);
    let (dst, src) = crate::protocol::SmtpCodec::new(io?).split();
    let handler = session_service.start();

    src.parse(SmtpParser)
        .with_connection(connection)
        // the steream is passed through the session handler and back
        .through(handler)
        // prevent polling after shutdown
        .fuse_shutdown()
        // prevent polling of completed stream
        .fuse()
        // forward to client
        .forward(dst)
        // prevent polling of completed forward
        .fuse()
        //.then(move |r| match r {
        //    Ok(_) => Ok(info!("connection {} gone", connection)),
        //    Err(e) => Err(warn!("connection {} gone with error {:?}", connection, e)),
        //})
        //.fuse()
        .await
}
