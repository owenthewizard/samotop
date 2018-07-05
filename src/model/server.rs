use service::TcpService;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[derive(Clone)]
pub struct SamotopServer<S>
where
    S: TcpService + Clone,
{
    pub addr: String,
    pub service: S,
}

#[derive(Clone)]
pub struct SamotopPort<S>
where
    S: TcpService + Clone,
{
    pub addr: SocketAddr,
    pub service: S,
}

pub struct SamotopListener<S>
where
    S: TcpService + Clone,
{
    pub listener: TcpListener,
    pub service: S,
}
