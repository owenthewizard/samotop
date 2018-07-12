use grammar::SmtpParser;
use model::controll::*;
use protocol::*;
use service::*;
use tokio::io;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio_codec::Decoder;
use util::*;

#[derive(Clone)]
pub struct SamotopService<S> {
    session_service: S,
}
impl<S> SamotopService<S> {
    pub fn new(session_service: S) -> Self {
        Self { session_service }
    }
}

impl<S, H> TcpService for SamotopService<S>
where
    S: SessionService<Handler = H>,
    H: Send + 'static,
    H: Sink<SinkItem = ServerControll, SinkError = io::Error>,
    H: Stream<Item = ClientControll, Error = io::Error>,
{
    type Future = Box<Future<Item = (), Error = ()> + Send>;
    fn handle(self, socket: TcpStream) -> Self::Future {
        let local = socket.local_addr().ok();
        let peer = socket.peer_addr().ok();
        info!("accepted peer {:?} on {:?}", peer, local);
        let (dst, src) = SmtpCodec::new().framed(socket).split();

        let task = src
            .peer(local, peer)
            .parse(SmtpParser)
            // the steream is teed into the session handler and back
            .tee(self.session_service.start())
            // prevent polling after shutdown
            .fuse_shutdown()
            // prevent polling of completed stream
            .fuse()
            // forward to client
            .forward(dst)
            .then(move |r| match r {
                Ok(_) => Ok(info!("peer {:?} gone from {:?}", peer, local)),
                Err(e) => Err(warn!("peer {:?} gone from {:?} with error {:?}", peer, local, e))
            }).fuse();

        Box::new(task)
    }
}
