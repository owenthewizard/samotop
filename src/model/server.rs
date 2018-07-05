use service::SamotopService;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[derive(Clone)]
pub struct SamotopServer<S>
where
    S: SamotopService + Clone,
{
    pub addr: String,
    pub factory: S,
}

#[derive(Clone)]
pub struct SamotopPort<S>
where
    S: SamotopService + Clone,
{
    pub addr: SocketAddr,
    pub factory: S,
}

pub struct SamotopListener<S>
where
    S: SamotopService + Clone,
{
    pub listener: TcpListener,
    pub factory: S,
}
