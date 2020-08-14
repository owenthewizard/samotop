use crate::common::*;
use crate::model::io::*;
use crate::model::Result;
use crate::service::tcp::TcpService;
use async_std::net::{TcpListener, TcpStream, ToSocketAddrs};
use futures::{
    future::{BoxFuture, TryFutureExt},
    stream::FuturesUnordered,
};
use std::net::SocketAddr;

/// `Server` takes care of accepting TCP connections and passing them to `TcpService` to `handle()`.
pub struct Server<'a> {
    ports: Vec<BoxFuture<'a, Result<Vec<SocketAddr>>>>,
}

impl<'a> Server<'a> {
    pub fn new() -> Self {
        Self { ports: vec![] }
    }
    pub fn on<N>(ports: N) -> Self
    where
        N: ToSocketAddrs + 'a,
        N::Iter: Send,
    {
        Self::new().and(ports)
    }
    pub fn on_all<I, N>(ports: I) -> Self
    where
        I: IntoIterator<Item = N>,
        N: ToSocketAddrs + 'a,
        N::Iter: Send,
    {
        Self::new().and_all(ports)
    }
    pub fn and_all<I, N>(mut self, ports: I) -> Self
    where
        I: IntoIterator<Item = N>,
        N: ToSocketAddrs + 'a,
        N::Iter: Send,
    {
        for port in ports.into_iter() {
            self = self.and(port);
        }
        self
    }
    pub fn and<N>(mut self, ports: N) -> Self
    where
        N: ToSocketAddrs + 'a,
        N::Iter: Send,
    {
        self.ports.push(Box::pin(Self::map_ports(ports)));
        self
    }
    fn map_ports(addrs: impl ToSocketAddrs) -> impl Future<Output = Result<Vec<SocketAddr>>> {
        addrs
            .to_socket_addrs()
            .map_ok(|i| i.into_iter().collect())
            .map_err(|e| e.into())
    }
    pub async fn resolve_ports(&mut self) -> Result<Vec<SocketAddr>> {
        let mut result = vec![];
        for port in self.ports.iter_mut() {
            let port = port.await?;
            result.extend_from_slice(&port[..]);
        }
        Ok(result)
    }
    pub async fn serve<S>(mut self: Server<'a>, service: S) -> Result<()>
    where
        S: TcpService<TcpStream> + Clone,
    {
        Self::serve_ports(service, self.resolve_ports().await?).await
    }
    async fn serve_ports<S>(service: S, addrs: impl IntoIterator<Item = SocketAddr>) -> Result<()>
    where
        S: TcpService<TcpStream> + Clone,
    {
        addrs
            .into_iter()
            .map(|a| Self::serve_port(service.clone(), a))
            .collect::<FuturesUnordered<_>>()
            .skip_while(|r| futures::future::ready(r.is_ok()))
            .take(1)
            .fold(Ok(()), |acc, cur| match cur {
                Err(e) => futures::future::err(e),
                Ok(()) => futures::future::ready(acc),
            })
            .await
    }
    async fn serve_port<S>(service: S, addr: SocketAddr) -> Result<()>
    where
        S: TcpService<TcpStream> + Clone,
    {
        trace!("Binding on {}", addr);
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| format!("Unable to bind {}: {}", addr, e))?;
        let mut incoming = listener.incoming();
        info!("Listening on {:?}", listener.local_addr());
        while let Some(stream) = incoming.next().await {
            let conn = if let Ok(ref stream) = stream {
                Connection::new(stream.local_addr().ok(), stream.peer_addr().ok())
            } else {
                Connection::default()
            };
            let stream = stream.map_err(|e| e.into());
            service.clone().handle(stream, conn).await;
        }
        Ok(())
    }
}
