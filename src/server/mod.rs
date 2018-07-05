pub mod builder;

use futures::stream;
use model::server::{SamotopListener, SamotopPort, SamotopServer};
use service::TcpService;
use std::net::ToSocketAddrs;
use tokio;
use tokio::io;
use tokio::net::TcpListener;
use tokio::prelude::*;

pub fn serve<S>(server: SamotopServer<S>) -> impl Future<Item = (), Error = ()>
where
    S: TcpService + Clone + Send + Sync + 'static,
{
    resolve(server)
        .map_err(|e| error!("{}", e))
        .for_each(|port| {
            tokio::spawn(bind(port).and_then(accept));
            Ok(())
        })

    //            tokio::spawn(accept(listener));
}

pub fn resolve<S>(server: SamotopServer<S>) -> impl Stream<Item = SamotopPort<S>, Error = io::Error>
where
    S: TcpService + Clone,
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

pub fn bind<S>(port: SamotopPort<S>) -> impl Future<Item = SamotopListener<S>, Error = ()>
where
    S: TcpService + Clone,
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

pub fn accept<S>(listener: SamotopListener<S>) -> impl Future<Item = (), Error = ()>
where
    S: TcpService + Clone,
{
    let SamotopListener { listener, service } = listener;
    let local = listener.local_addr().ok();
    listener
        .incoming()
        .map_err(move |e| error!("error accepting on {:?}: {:?}", local, e))
        .for_each(move |socket| Ok(service.clone().handle(socket)))
        .then(move |result| match result {
            Ok(_) => {
                info!("done accepting on {:?}, {:?}", local, result);
                Ok(())
            }
            Err(e) => {
                error!("done accepting on {:?} with error: {:?}", result, e);
                Err(())
            }
        })
}
