use futures::stream;
use model::server::{SamotopListener, SamotopPort, SamotopServer};
use server::builder::SamotopBuilder;
use service::samotop::SamotopService;
use service::TcpService;
use std::net::ToSocketAddrs;
use tokio;
use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;

/// Create a builder that can configure a samotop server and make it runnable as a task.
/// Each listener is executed as a separate task, but they are all joined into one future.
///
/// Example of creating a samotop server task (`Future<Item=(),Error=()>`):
/// ```
///     samotop::builder()
///              // SamotopService is the default, but you can set your own name here.
///             .with(samotop::service::samotop::SamotopService::new("MySamotop"))
///             .on("1.1.1.1:25")
///             .as_task();
/// ```
pub fn builder() -> SamotopBuilder<SamotopService> {
    SamotopBuilder::new("localhost:25", SamotopService::new("Samotop"))
}

/// Start the server, spawning each listener as a separate task.
pub(crate) fn serve<S>(server: SamotopServer<S>) -> impl Future<Item = (), Error = ()>
where
    S: Clone + Send + 'static,
    S: TcpService,
    S::Handler: Sink<SinkItem = TcpStream, SinkError = io::Error>,
    S::Handler: Send,
{
    resolve(server)
        .map_err(|e| error!("{}", e))
        .for_each(|port| tokio::spawn(bind(port).and_then(accept)))
}

/// Resolve `SamotopServer` addr into `SamotopPort`s
pub(crate) fn resolve<S>(
    server: SamotopServer<S>,
) -> impl Stream<Item = SamotopPort<S>, Error = io::Error>
where
    S: Clone,
    S: TcpService,
    S::Handler: Sink<SinkItem = TcpStream, SinkError = io::Error>,
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
pub(crate) fn bind<S>(port: SamotopPort<S>) -> impl Future<Item = SamotopListener<S>, Error = ()>
where
    S: Clone,
    S: TcpService,
    S::Handler: Sink<SinkItem = TcpStream, SinkError = io::Error>,
{
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
                service: service.clone(),
            })
        })
}

/// Accept incomming TCP connections and forward them to the handler sink created by TcpService
pub(crate) fn accept<S>(listener: SamotopListener<S>) -> impl Future<Item = (), Error = ()>
where
    S: TcpService,
    S::Handler: Sink<SinkItem = TcpStream, SinkError = io::Error>,
{
    let SamotopListener { listener, service } = listener;
    let local = listener.local_addr().ok();
    let handler = service.start();
    listener
        .incoming()
        .forward(handler)
        .then(move |result| match result {
            Ok((_i, _h)) => Ok(info!("done accepting on {:?}", local)),
            Err(e) => Err(error!("done accepting on {:?} with error: {:?}", local, e)),
        })
}
