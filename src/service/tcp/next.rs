use bytes::Bytes;
use grammar::SmtpParser;
use protocol::*;
use service::TcpServiceNext;
use service::*;
use tokio::io;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio_codec::Decoder;

#[derive(Clone)]
pub struct SamotopService<M> {
    mail_service: M,
}
impl<M> SamotopService<M> {
    pub fn new(mail_service: M) -> Self {
        Self { mail_service }
    }
}

impl<M> TcpServiceNext for SamotopService<M>
where
    M: MailService + Send + 'static,
    M::MailDataWrite: Sink<SinkItem = Bytes, SinkError = io::Error> + Send,
{
    type Future = Box<Future<Item = (), Error = ()> + Send>;
    fn handle(self, socket: TcpStream) -> Self::Future {
        trace!("Got a stream!");

        let local = socket.local_addr().ok();
        let peer = socket.peer_addr().ok();
        info!("accepted peer {:?} on {:?}", peer, local);
        let (dst, src) = SmtpCodec::new().framed(socket).split();
        info!("got an item");

        let task = src
            .peer(local, peer)
            .parse(SmtpParser)
            .mail(self.mail_service)
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
