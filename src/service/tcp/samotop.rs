use bytes::Bytes;
use futures::StartSend;
use service::*;
use protocol::*;
use service::mail::ConsoleMail;
use grammar::SmtpParser;
use tokio::io;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio_codec::Decoder;

#[doc = "A TCP service providing SMTP - Samotop"]
#[derive(Clone, Copy)]
pub struct SamotopService<M> {
    mail_service: M
}

pub fn default()->SamotopService<ConsoleMail> {
    SamotopService::new(ConsoleMail::default())
}

impl<M> SamotopService <M> {
    pub fn new(mail_service:M) -> Self {
        Self { mail_service }
    }
    pub fn serve<MX>(self, mail_service:MX) -> SamotopService<MX>{
        SamotopService::new(mail_service)
    }
}

impl<M> TcpService for SamotopService<M> where M:Clone{
    type Handler = SamotopHandler<M>;
    fn start(&self) -> Self::Handler {
        SamotopHandler::new(self.mail_service.clone())
    }
}

pub struct SamotopHandler<M> {
    mail_service: M,
    pending: Box<Future<Item = (), Error = io::Error> + Send + 'static>,
}

impl<M> SamotopHandler<M> {
    pub fn new(mail_service:M) -> Self {
        Self {
            mail_service,
            pending: Box::new(future::ok(())),
        }
    }
}

impl<M> Sink for SamotopHandler<M>
 where M:MailService +Clone +Send +'static,
    M::MailDataWrite:Sink<SinkItem = Bytes, SinkError = io::Error> + Send
    ,M::MailDataWrite:MailHandler
  {
    type SinkItem = TcpStream;
    type SinkError = io::Error;

    fn start_send(&mut self, socket: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        let local = socket.local_addr().ok();
        let peer = socket.peer_addr().ok();
        info!("accepted peer {:?} on {:?}", peer, local);
        let (dst, src) = SmtpCodec::new().framed(socket).split();
        info!("got an item");

        let task = src
            .peer(local, peer)
            .parse(SmtpParser)
            .mail(self.mail_service.clone())
            // prevent polling after shutdown
            .fuse_shutdown()
            // prevent polling of completed stream
            .fuse()            
            // forward to client
            .forward(dst)
            .then(move |r| match r {
                Ok(_)=>Ok(info!("peer {:?} gone from {:?}", peer, local)),
                Err(e) => {
                    warn!("peer {:?} gone from {:?} with error {:?}", peer, local, e);
                    Err(e)
                }
            }).fuse();

        self.pending = Box::new(task);
        Ok(AsyncSink::Ready)
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.pending.poll()
    }
}
