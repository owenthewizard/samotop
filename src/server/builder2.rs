use model::next::SamotopServer;
use server::next as server;
use service::TcpService2;
use tokio::io;
use tokio::net::TcpStream;
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
    pub fn as_task(self) -> impl Future<Item = (), Error = ()>
    where
        S: TcpService2 + Clone + Send + 'static,
        S::Handler: Send,
        S::Handler: Sink<SinkItem = TcpStream, SinkError = io::Error>,
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
        future::join_all(ports.into_iter().map(move |addr| {
            server::serve(SamotopServer {
                addr,
                service: service.clone(),
            })
        })).map(|_| ())
    }
}
