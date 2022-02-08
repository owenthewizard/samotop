mod io;
mod tcp;
#[cfg(unix)]
mod unix;

pub use self::io::*;
pub use self::tcp::*;
#[cfg(unix)]
pub use self::unix::*;

use super::{Server, ServerService};
use crate::{
    common::*,
    config::{Component, ServerContext},
    io::{HandlerService, Session},
};
use futures_util::{
    stream::{select_all, FuturesUnordered},
    FutureExt, StreamExt, TryStreamExt,
};

impl crate::config::ComposableComponent for ServerService {
    fn from_none() -> Self::Target {
        Self::from_many(vec![])
    }

    fn from_many(options: Vec<Self::Target>) -> Self::Target {
        Arc::new(options)
    }
}

impl Server for Vec<<ServerService as Component>::Target> {
    fn sessions<'s, 'f>(
        &'s self,
    ) -> S1Fut<'f, Result<Pin<Box<dyn Stream<Item = Result<Session>> + Send + Sync>>>>
    where
        's: 'f,
    {
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

impl ServerContext {
    pub async fn serve(mut self) -> Result<()> {
        let server = self.store.get_or_compose::<ServerService>().clone();
        let handler = self.store.get_or_compose::<HandlerService>().clone();

        server
            .sessions()
            .await?
            .for_each_concurrent(1000, move |session| match session {
                Ok(mut session) => {
                    let handler = handler.clone();
                    Box::pin(async move {
                        handler
                            .handle(&mut session)
                            .map(|result| {
                                if let Err(e) = result {
                                    warn!("Session failed: {}", e);
                                }
                            })
                            .await
                    })
                }
                Err(e) => {
                    warn!("Session not accepted: {}", e);
                    Box::pin(ready(())) as S3Fut<()>
                }
            })
            .await;

        Ok(())
    }
}
