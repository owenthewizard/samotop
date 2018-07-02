pub mod builder;

use futures::stream;
use model::server::{SamotopListener, SamotopPort, SamotopServer};
use service::SamotopService;
use std::net::ToSocketAddrs;
use tokio;
use tokio::io;
use tokio::net::TcpListener;
use tokio::prelude::*;

pub fn serve<S>(server: SamotopServer<S>) -> impl Future<Item = (), Error = ()>
where
    S: SamotopService + Clone + Send + Sync + 'static,
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
    S: SamotopService + Clone,
{
    let SamotopServer { addr, factory } = server;
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
            factory: factory.clone(),
        })
}

pub fn bind<S>(port: SamotopPort<S>) -> impl Future<Item = SamotopListener<S>, Error = ()>
where
    S: SamotopService + Clone,
{
    let SamotopPort {
        addr: local,
        factory,
    } = port;
    future::lazy(move || future::result(TcpListener::bind(&local)))
        .map_err(move |e| error!("bind error on {}: {}", local, e))
        .and_then(move |listener| {
            info!("listening on {}", local);
            future::ok(SamotopListener {
                listener,
                factory: factory.clone(),
            })
        })
}

pub fn accept<S>(listener: SamotopListener<S>) -> impl Future<Item = (), Error = ()>
where
    S: SamotopService + Clone,
{
    let SamotopListener { listener, factory } = listener;
    let local = listener.local_addr().ok();
    listener
        .incoming()
        .map_err(move |e| error!("error accepting on {:?}: {:?}", local, e))
        .for_each(move |socket| {
            let local = socket.local_addr().ok();
            let peer = socket.peer_addr().ok();
            info!("accepted peer {:?} on {:?}", peer, local);

            factory.clone().handle(socket);
            Ok(())
        })
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

/*
pub fn bind(port: SamotopPort<S>) -> impl Future<Item = (), Error = ()> {
    let addr_dbg1 = format!("{:?}", addr);
    let addr_dbg2 = format!("{:?}", addr);
    stream::once(TcpListener::bind(&addr))
        .map_err(move |e| error!("bind error on {}: {}", addr_dbg1, e))
        .for_each(move |listener| {
            info!("listening on {}", addr_dbg2);
            tokio::spawn(accept(listener));
            Ok(())
        })
}

pub fn accept(listener: TcpListener) -> impl Future<Item = (), Error = ()> {
    let local = listener.local_addr().ok();
    listener
        .incoming()
        .map_err(move |e| error!("error accepting on {:?}: {:?}", local, e))
        .for_each(move |socket| {
            let local = socket.local_addr().ok();
            let peer = socket.peer_addr().ok();
            info!("accepted peer {:?} on {:?}", peer, local);

            let (sink, stream) = protocol::SmtpCodec::new(peer, local).framed(socket).split();

            stream
                .map(answers)
                .flatten()
                .forward(sink)
                .then(move |result| match result {
                    Ok(_) => {
                        info!("peer {:?} gone from {:?}", peer, local);
                        Ok(())
                    }
                    Err(e) => {
                        error!("peer {:?} gone from {:?} with error {:?}", peer, local, e);
                        Err(())
                    }
                })
        })
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
*/
