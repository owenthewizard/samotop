use futures::future;
use model::server::SamotopServer;
use server;
use service::SamotopService;
use tokio::prelude::Future;

pub struct Samotop<S>
where
    S: SamotopService + Clone,
{
    pub default_port: &'static str,
    pub default_service: S,
}

impl<S> Samotop<S>
where
    S: SamotopService + Clone,
{
    pub fn with<SX>(&self, factory: SX) -> SamotopBuilder<SX>
    where
        SX: SamotopService + Clone + Send + Sync + 'static,
    {
        SamotopBuilder::new(self.default_port.into(), factory)
    }
}

#[derive(Clone)]
pub struct SamotopBuilder<S>
where
    S: SamotopService + Clone,
{
    default_port: String,
    ports: Vec<String>,
    factory: S,
}

impl<S> SamotopBuilder<S>
where
    S: SamotopService + Clone + Send + Sync + 'static,
{
    pub fn new(default_port: String, factory: S) -> Self {
        Self {
            default_port,
            ports: vec![],
            factory,
        }
    }
    pub fn with<SX>(self, factory: SX) -> SamotopBuilder<SX>
    where
        SX: SamotopService + Clone,
    {
        let Self {
            default_port,
            ports,
            ..
        } = self;
        SamotopBuilder {
            default_port,
            factory,
            ports,
        }
    }
    pub fn on(self, port: impl ToString) -> Self {
        let mut me = self.clone();
        me.ports.push(port.to_string());
        me
    }
    pub fn on_all<P>(self, ports: impl IntoIterator<Item = P>) -> Self
    where
        P: ToString,
    {
        let mut me = self.clone();
        me.ports
            .extend(ports.into_iter().map(|port| port.to_string()));
        me
    }
    pub fn as_task(self) -> impl Future<Item = (), Error = ()> {
        let Self {
            default_port,
            ports,
            factory,
        } = self;
        let ports = match ports.len() {
            0 => vec![default_port],
            _ => ports,
        };
        future::join_all(ports.into_iter().map(move |addr| {
            server::serve(SamotopServer {
                addr,
                factory: factory.clone(),
            })
        })).map(|_| ())
    }
}
