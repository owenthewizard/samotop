use futures::future;
use model::server::SamotopServer;
use server;
use service::TcpService;
use tokio::prelude::Future;

#[derive(Clone)]
pub struct SamotopBuilder<S>
where
    S: TcpService + Clone,
{
    default_port: String,
    ports: Vec<String>,
    service: S,
}

impl<S> SamotopBuilder<S>
where
    S: TcpService + Clone + Send + Sync + 'static,
{
    pub fn new(default_port: impl ToString, service: S) -> Self {
        Self {
            default_port: default_port.to_string(),
            ports: vec![],
            service,
        }
    }
    pub fn with<SX>(self, service: SX) -> SamotopBuilder<SX>
    where
        SX: TcpService + Clone,
    {
        let Self {
            default_port,
            ports,
            ..
        } = self;
        SamotopBuilder {
            default_port,
            service,
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
            service,
        } = self;
        let ports = match ports.len() {
            0 => vec![default_port],
            _ => ports,
        };
        future::join_all(ports.into_iter().map(move |addr| {
            server::serve(SamotopServer {
                addr,
                service: service.clone(),
            })
        })).map(|_| ())
    }
}
