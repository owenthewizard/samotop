use crate::model::server::SamotopServer;
use crate::server;
use crate::service::TcpService;
use tokio::prelude::*;

#[derive(Clone)]
pub struct SamotopBuilder<S> {
    default_port: String,
    ports: Vec<String>,
    service: S,
}

impl<S> SamotopBuilder<S> {
    pub fn new(default_port: impl ToString, service: S) -> Self {
        Self {
            default_port: default_port.to_string(),
            ports: vec![],
            service,
        }
    }
    pub fn with<SX>(self, service: SX) -> SamotopBuilder<SX> {
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
    pub fn on(self, port: impl ToString) -> Self
    where
        S: Clone,
    {
        let mut me = self.clone();
        me.ports.push(port.to_string());
        me
    }
    pub fn on_all<P>(self, ports: impl IntoIterator<Item = P>) -> Self
    where
        P: ToString,
        S: Clone,
    {
        let mut me = self.clone();
        me.ports
            .extend(ports.into_iter().map(|port| port.to_string()));
        me
    }
    pub fn into_servers(self) -> impl Iterator<Item = SamotopServer<S>>
    where
        S: Clone,
    {
        let Self {
            default_port,
            ports,
            service,
        } = self;
        let ports = match ports.len() {
            0 => vec![default_port],
            _ => ports,
        };
        ports.into_iter().map(move |addr| SamotopServer {
            addr,
            service: service.clone(),
        })
    }
    pub fn build_task<Fut>(self) -> impl Future<Item = (), Error = ()>
    where
        S: TcpService<Future = Fut> + Clone + Send + 'static,
        Fut: Future<Item = (), Error = ()> + Send + 'static,
    {
        future::join_all(self.into_servers().map(server::serve)).map(|_| ())
    }
}
