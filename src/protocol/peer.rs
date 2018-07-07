use model::controll::ServerControll;
use std::net::SocketAddr;
use tokio::prelude::*;
use util::futu::*;

pub trait HasPeer
where
    Self: Sized,
{
    fn peer(self, local: Option<SocketAddr>, peer: Option<SocketAddr>) -> WithPeer<Self> {
        WithPeer {
            stream: self,
            local,
            peer,
            connected: false,
            shutdown: false,
        }
    }
}

impl<S> HasPeer for S
where
    S: Stream,
{
}

pub struct WithPeer<S> {
    stream: S,
    local: Option<SocketAddr>,
    peer: Option<SocketAddr>,
    connected: bool,
    shutdown: bool,
}

impl<S> Stream for WithPeer<S>
where
    S: Stream<Item = ServerControll>,
{
    type Item = ServerControll;
    type Error = S::Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        if !self.connected {
            self.connected = true;
            return ok(ServerControll::PeerConnected {
                local: self.local,
                peer: self.peer,
            });
        }

        match try_ready!(self.stream.poll()) {
            None => match self.shutdown {
                true => none(),
                false => {
                    self.shutdown = true;
                    ok(ServerControll::PeerShutdown)
                }
            },
            Some(c) => ok(c),
        }
    }
}
