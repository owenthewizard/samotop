use futures::stream;
use crate::model::server::{SamotopListener, SamotopPort, SamotopServer};
use crate::server::builder::SamotopBuilder;
use crate::service::mail::ConsoleMail;
use crate::service::session::StatefulSessionService;
use crate::service::tcp::SamotopService;
use crate::service::TcpService;
use std::net::ToSocketAddrs;
use tokio;
use tokio::io;
use tokio::net::TcpListener;
use tokio::prelude::*;

/// Create a builder that can configure a samotop server and make it runnable as a task.
/// Each listener is executed as a separate task, but they are all joined into one future.
///
/// Example of creating a samotop server task (`Future<Item=(),Error=()>`):
/// ```
///     samotop::builder()
///             .on("1.1.1.1:25")
///             .build_task();
/// ```
pub fn builder() -> SamotopBuilder<SamotopService<StatefulSessionService<ConsoleMail>>> {
    let mail_svc = ConsoleMail::new("Samotop STARTTLS");
    let session_svc = StatefulSessionService::new(mail_svc);
    let tcp_svc = SamotopService::new(session_svc, Default::default());
    SamotopBuilder::new("localhost:12345", tcp_svc)
}

/// Resolve `SamotopServer` addr into `SamotopPort`s
pub(crate) fn resolve<S>(
    server: SamotopServer<S>,
) -> impl Stream<Item = SamotopPort<S>, Error = io::Error>
where
    S: Clone,
{
    let SamotopServer { addr, service } = server;
    stream::once(addr.to_socket_addrs())
        .map(stream::iter_ok)
        .map_err(move |e| {
            io::Error::new(
                e.kind(),
                format!("Cannot reslove socket address {}: {}", addr, e),
            )
        })
        .flatten()
        .map(move |addr| SamotopPort {
            addr,
            service: service.clone(),
        })
}

// Bind the samotop TCP port
pub(crate) fn bind<S>(port: SamotopPort<S>) -> impl Future<Item = SamotopListener<S>, Error = ()> {
    let SamotopPort {
        addr: local,
        service,
    } = port;
    future::lazy(move || future::result(TcpListener::bind(&local)))
        .map_err(move |e| error!("bind error on {}: {}", local, e))
        .and_then(move |listener| {
            info!("listening on {}", local);
            future::ok(SamotopListener {
                listener,
                service: service,
            })
        })
}

/// Start the server, spawning each listener as a separate task.
pub(crate) fn serve<S, Fut>(server: SamotopServer<S>) -> impl Future<Item = (), Error = ()>
where
    S: Clone + Send + 'static,
    S: TcpService<Future = Fut>,
    Fut: Future<Item = (), Error = ()> + Send + 'static,
{
    resolve(server)
        .map(|port| {
            let SamotopPort { addr, service } = port;
            let service = stream::repeat(service);
            SamotopPort { addr, service }
        })
        .map_err(|e| error!("{}", e))
        .for_each(|port| tokio::spawn(bind(port).and_then(accept)))
}

/// Accept incomming TCP connections and forward them to the handler sink created by TcpService
pub(crate) fn accept<S, TcpSvc, Fut>(
    listener: SamotopListener<S>,
) -> impl Future<Item = (), Error = ()>
where
    S: Stream<Item = TcpSvc, Error = io::Error>,
    TcpSvc: TcpService<Future = Fut>,
    Fut: Future<Item = (), Error = ()> + Send + 'static,
{
    let SamotopListener { listener, service } = listener;
    let local = listener.local_addr().ok();
    listener
        .incoming()
        .zip(service)
        .for_each(|(tcp, handler)| {
            tokio::spawn(handler.handle(tcp));
            Ok(())
        })
        .then(move |result| match result {
            Ok(_) => Ok(info!("done accepting on {:?}", local)),
            Err(e) => Err(error!("done accepting on {:?} with error: {:?}", local, e)),
        })
}
