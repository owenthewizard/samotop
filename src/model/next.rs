use service::TcpService;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[derive(Clone)]
pub struct SamotopServer<S>
{
    pub addr: String,
    pub service: S,
}

#[derive(Clone)]
pub struct SamotopPort<S>
{
    pub addr: SocketAddr,
    pub service: S,
}

pub struct SamotopListener<S>
{
    pub listener: TcpListener,
    pub service: S,
}
