use super::ServerService;
use super::{Server, Session};
use crate::config::ServerContext;
use crate::config::Setup;
use crate::common::*;
use crate::io::*;
use async_std::os::unix::net::UnixStream;
use async_std::{os::unix::net::UnixListener, path::PathBuf as SocketAddr};
use futures_core::Stream;
use futures_util::stream::FuturesUnordered;
use futures_util::{TryFutureExt, TryStreamExt};

/// `UnixServer` takes care of accepting Unix socket connections and passing them to an `IoService` to `handle()`.
#[derive(Default, Debug, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct UnixServer<T = SocketAddr> {
    port: T,
}

impl<T> UnixServer<T> {
    /// Listen on this port - usually addres:port. You can call this multiple times to listen on multiple ports.
    pub fn on(port: T) -> Self {
        Self { port }
    }
}

impl<T> Setup for UnixServer<T>
where
    T: Into<SocketAddr> + Unpin + Clone + Send + Sync + 'static,
{
    fn setup(&self, ctx: &mut ServerContext) {
        ctx.store.add::<ServerService>(Arc::new(self.clone()))
    }
}

impl<T> Server for UnixServer<T>
where
    T: Into<SocketAddr> + Unpin + Clone + Send + Sync + 'static,
{
    fn sessions<'s, 'f>(
        &'s self,
    ) -> S1Fut<'f, Result<Pin<Box<dyn Stream<Item = Result<Session>> + Send + Sync>>>>
    where
        's: 'f,
    {
        let port = self.port.clone();
        Box::pin(async move {
            let addrs = vec![port.into()].into_iter();

            let listeners = addrs
                .map(|address| {
                    trace!("Binding on {:?}", address);
                    UnixListener::bind(address.clone())
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
    accepts: FuturesUnordered<S2Fut<'static, (Result<UnixStream>, UnixListener)>>,
}

impl Accepting {
    pub fn new(listeners: Vec<UnixListener>) -> Self {
        let mut me = Self {
            accepts: FuturesUnordered::default(),
        };
        for l in listeners {
            me.accept(l)
        }
        me
    }
    fn accept(&mut self, listener: UnixListener) {
        self.accepts.push(Box::pin(async move {
            (
                listener.accept().await.map(|v| v.0).map_err(|e| e.into()),
                listener,
            )
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
            Some((Ok(stream), listener)) => {
                self.accept(listener);

                let conn = ConnectionInfo::new(
                    stream
                        .local_addr()
                        .map(|s| {
                            s.as_pathname()
                                .and_then(|p| p.to_str())
                                .map(|s| s.to_owned())
                        })
                        .ok()
                        .flatten()
                        .unwrap_or_default(),
                    stream
                        .peer_addr()
                        .map(|s| {
                            s.as_pathname()
                                .and_then(|p| p.to_str())
                                .map(|s| s.to_owned())
                        })
                        .ok()
                        .flatten()
                        .unwrap_or_default(),
                );

                let mut session = Session::new(stream);
                session.store.set::<ConnectionInfo>(conn);
                Poll::Ready(Some(Ok(session)))
            }
        }
    }
}
