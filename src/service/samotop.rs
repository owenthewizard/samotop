use futures::StartSend;
use service::TcpService;
use protocol::*;
use service::console::ConsoleMail;
use grammar::SmtpParser;
use tokio::io;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio_codec::Decoder;

#[doc = "A TCP service providing SMTP - Samotop"]
#[derive(Clone)]
pub struct SamotopService {
    name: String,
}

impl SamotopService {
    pub fn new(name: impl ToString) -> Self {
        Self { name: name.to_string() }
    }
}

impl TcpService for SamotopService {
    type Handler = SamotopHandler;
    fn start(&self) -> Self::Handler {
        SamotopHandler::new(&self.name)
    }
}

pub struct SamotopHandler {
    name: String,
    pending: Box<Future<Item = (), Error = io::Error> + Send + 'static>,
}

impl SamotopHandler {
    pub fn new(name: impl ToString) -> Self {
        SamotopHandler {
            name: name.to_string(),
            pending: Box::new(future::ok(())),
        }
    }
}

impl Sink for SamotopHandler {
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
            .mail(ConsoleMail::new(Some(self.name.clone())))
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
