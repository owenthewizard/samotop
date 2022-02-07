use crate::{
    common::*,
    io::Io,
    store::{Component, MultiComponent, Store},
};

#[cfg(feature = "server")]
mod tcp;
#[cfg(feature = "server")]
pub use self::tcp::*;

#[cfg(all(unix, feature = "server"))]
mod unix;
#[cfg(all(unix, feature = "server"))]
pub use self::unix::*;

#[cfg(all(unix, feature = "server"))]
mod io;
#[cfg(all(unix, feature = "server"))]
pub use self::io::*;

pub trait Server {
    fn sessions<'s, 'f>(
        &'s self,
    ) -> S1Fut<'f, Result<Pin<Box<dyn Stream<Item = Result<Session>> + Send + Sync>>>>
    where
        's: 'f;
}

pub struct Session {
    pub io: Box<dyn Io>,
    pub store: Store,
}
impl Session {
    pub fn new(io: impl Io + 'static) -> Self {
        Self {
            io: Box::new(io),
            store: Store::default(),
        }
    }
}

pub struct ServerService {}
impl Component for ServerService {
    type Target = Arc<dyn Server + Send + Sync>;
}
impl MultiComponent for ServerService {}

#[cfg(all(unix, feature = "server"))]
impl crate::store::ComposableComponent for ServerService {
    fn compose<'a, I>(options: I) -> Self::Target
    where
        I: Iterator<Item = &'a Self::Target> + 'a,
        Self::Target: Clone + 'a,
    {
        Arc::new(options.cloned().collect::<Vec<_>>())
    }
}

#[cfg(all(unix, feature = "server"))]
impl Server for Vec<<ServerService as Component>::Target> {
    fn sessions<'s, 'f>(
        &'s self,
    ) -> S1Fut<'f, Result<Pin<Box<dyn Stream<Item = Result<Session>> + Send + Sync>>>>
    where
        's: 'f,
    {
        use futures_util::{
            stream::{select_all, FuturesUnordered},
            TryStreamExt,
        };
        Box::pin(async move {
            let streams = FuturesUnordered::default();
            for srv in self {
                streams.push(srv.sessions())
            }

            let sessions = streams.try_collect::<Vec<_>>().await?;
            let sessions = select_all(sessions);
            Ok(Box::pin(sessions)
                as Pin<
                    Box<dyn Stream<Item = Result<Session>> + Send + Sync>,
                >)
        })
    }
}
