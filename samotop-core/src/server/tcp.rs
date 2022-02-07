use super::ServerService;
use super::{Server, Session};
use crate::builder::{ServerContext, Setup};
use crate::common::*;
use crate::io::*;
use async_std::net::{TcpListener, TcpStream, ToSocketAddrs};
use futures_core::Stream;
use futures_util::stream::FuturesUnordered;
use futures_util::{TryFutureExt, TryStreamExt};
use std::net::SocketAddr;

/// `TcpServer` takes care of accepting TCP connections and passing them to an `IoService` to `handle()`.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct TcpServer<T = String> {
    port: T,
}

impl<T> TcpServer<T> {
    /// Listen on this port - usually addres:port. You can call this multiple times to listen on multiple ports.
    pub fn on(port: T) -> Self {
        Self { port }
    }
}

impl<'a, T> Setup for TcpServer<T>
where
    T: ToSocketAddrs + Unpin + Clone + Send + Sync + 'static,
    T::Iter: Send + Sync,
{
    fn setup(&self, ctx: &mut ServerContext) {
        ctx.store.add::<ServerService>(Arc::new(self.clone()))
    }
}

impl<T> Server for TcpServer<T>
where
    T: ToSocketAddrs + Unpin + Clone + Send + Sync + 'static,
    T::Iter: Send + Sync,
{
    fn sessions<'s, 'f>(
        &'s self,
    ) -> S1Fut<'f, Result<Pin<Box<dyn Stream<Item = Result<Session>> + Send + Sync>>>>
    where
        's: 'f,
    {
        let port = self.port.clone();

        Box::pin(async move {
            let addrs = port
                .to_socket_addrs()
                //.map_ok(|addrs| addrs.collect::<Vec<_>>())
                .await?;

            let listeners = addrs
                .map(|address| {
                    trace!("Binding on {:?}", address);
                    TcpListener::bind(address)
                        .map_err(move |e| format!("Unable to bind {:?}: {}", address, e))
                })
                .collect::<FuturesUnordered<_>>()
                .try_collect::<Vec<_>>()
                .await?;

            let sessions = Accepting::new(listeners);

            //let sessions = Sessions::on(port).await?;
            Ok(Box::pin(sessions)
                as Pin<
                    Box<dyn Stream<Item = Result<Session>> + Send + Sync>,
                >)
        })
    }
}

struct Accepting {
    accepts: FuturesUnordered<S2Fut<'static, (Result<(TcpStream, SocketAddr)>, TcpListener)>>,
}

impl Accepting {
    pub fn new(listeners: Vec<TcpListener>) -> Self {
        let mut me = Self {
            accepts: FuturesUnordered::default(),
        };
        for l in listeners {
            me.accept(l)
        }
        me
    }
    fn accept(&mut self, listener: TcpListener) {
        self.accepts.push(Box::pin(async move {
            (listener.accept().await.map_err(|e| e.into()), listener)
        }))
    }
}

impl Stream for Accepting {
    type Item = Result<Session>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match ready!(Pin::new(&mut self.as_mut().get_mut().accepts).poll_next(cx)) {
            None => Poll::Ready(None),
            Some((Err(e), listener)) => {
                let res = Poll::Ready(Some(Err(format!(
                    "Failed to accept on {:?}:{}",
                    listener.local_addr(),
                    e
                )
                .into())));
                self.accept(listener);

                res
            }
            Some((Ok((stream, _addr)), listener)) => {
                self.accept(listener);

                let conn = ConnectionInfo::new(
                    stream
                        .local_addr()
                        .map(|s| s.to_string())
                        .unwrap_or_default(),
                    stream
                        .peer_addr()
                        .map(|s| s.to_string())
                        .unwrap_or_default(),
                );

                let mut session = Session::new(stream);
                session.store.set::<ConnectionInfo>(conn);
                Poll::Ready(Some(Ok(session)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::builder::Builder;

    use super::*;

    #[test]
    fn use_samotop_server() {
        let _ = TcpServer::<&str>::default();
    }
    #[test]
    fn builder_builds_task() {
        let mail = Builder::default() + TcpServer::on("localhost:25252");
        let _fut = mail.build();
    }
}
