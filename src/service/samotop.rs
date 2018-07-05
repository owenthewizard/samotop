
use protocol::*;
use service::TcpService;
use service::console::ConsoleMail;
use grammar::SmtpParser;
use tokio;
use tokio::io;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio_codec::Decoder;

/** A TCP service providing SMTP - Samotop */
#[derive(Clone)]
pub struct SamotopService {
    name: String,
    parser: SmtpParser,
}

impl SamotopService {
    pub fn new(name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            parser: SmtpParser,
        }
    }
}

impl TcpService for SamotopService {
    fn handle(self, socket: TcpStream) {
        let local = socket.local_addr().ok();
        let peer = socket.peer_addr().ok();
        info!("accepted peer {:?} on {:?}", peer, local);
        let (dst, src) = SmtpCodec::new().framed(socket).split();
        let task = src
            .peer(peer)
            .parse(SmtpParser)
            .mail(ConsoleMail::new(Some(self.name)))
            // prevent polling after shutdown
            .fuse_shutdown()
            // prevent polling of completed stream
            .fuse()            
            // forward to client
            .forward(dst)
            .map(move |_| info!("peer {:?} gone from {:?}", peer, local))
            .map_err(move|e:io::Error| warn!("peer {:?} gone from {:?} with error {:?}", peer, local, e));

        tokio::spawn(task);
    }
}
