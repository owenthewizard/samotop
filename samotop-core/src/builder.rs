use crate::common::*;
use crate::store::Store;
use std::ops::{Add, AddAssign};

/**
Can set up the given mail services.

```
# use samotop_core::common::*;
# use samotop_core::mail::*;
# use samotop_core::builder::*;
/// This mail setup replaces dispatch service with default. No mail will be sent.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
struct NoDispatch;

impl Setup for NoDispatch
{
    fn setup(&self, ctx: &mut ServerContext) {
        ctx.store.add::<MailDispatchService>(Arc::new(NullDispatch))
    }
}

let mail_svc = Builder::default() + NoDispatch;

```
*/

pub trait Setup {
    fn setup(&self, ctx: &mut ServerContext);
}

/// Builds a server from components with a builder pattern.
///
/// Add components with a + sign. The component must implement `Setup`
/// Finally call `build()` which produces a server future
#[derive(Default, Debug)]
pub struct Builder {
    setup: Vec<Box<dyn BuildingBlock>>,
}

/// A Server setup with extra requirements for the Builder
pub trait BuildingBlock: Setup + fmt::Debug + 'static {}

impl<T> BuildingBlock for T where T: Setup + fmt::Debug + 'static {}

/// Context of the server being setup for a run.
#[derive(Default, Debug)]
pub struct ServerContext {
    pub store: Store,
}

/// Composing a mail service with +
impl<T: BuildingBlock> Add<T> for Builder {
    type Output = Self;
    /// Add given mail setup to the service configuration
    fn add(mut self, setup: T) -> Self::Output {
        self += setup;
        self
    }
}
/// Composing a mail service with +=
impl<T: BuildingBlock> AddAssign<T> for Builder {
    fn add_assign(&mut self, setup: T) {
        trace!("Service builder using setup {:?}", setup);
        self.setup.push(Box::new(setup))
    }
}

impl Builder {
    /// Start with empty configuration
    pub fn new() -> Builder {
        Builder::default()
    }
    /// Use a given MailSetup to build a MailService.
    ///
    /// See MailSetup for examples. Prefer to build with the + sign.
    pub fn using(self, setup: impl BuildingBlock) -> Self {
        self + setup
    }

    /// Finalize and produce the MailService.
    pub fn build(&self) -> ServerContext {
        let mut context = ServerContext::default();
        for setup in self.setup.iter() {
            setup.setup(&mut context)
        }
        context
    }
}

#[cfg(feature = "runner")]
impl ServerContext {
    pub async fn run(self) -> Result<()> {
        use crate::io::HandlerService;
        use crate::server::ServerService;
        use futures_util::FutureExt;
        use futures_util::StreamExt;

        let server = self.store.get_or_compose::<ServerService>();
        let handler = self.store.get_or_compose::<HandlerService>();

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
