
use model::controll::*;
use protocol::*;
use service::{console::ConsoleMail, SamotopService};
use grammar::SmtpParser;
use tokio;
use tokio::io;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio_codec::Decoder;

#[derive(Clone)]
pub struct MailService {
    name: String, 
    parser: SmtpParser,
}

impl MailService {
    pub fn new(name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            parser: SmtpParser,
        }
    }
}

impl SamotopService for MailService {
    fn handle(self, socket: TcpStream) {
        let local = socket.local_addr().ok();
        let peer = socket.peer_addr().ok();
        let (dst, src) = SmtpCodec::new().framed(socket).split();
        let task = src
            .peer(peer)
            .parse(SmtpParser)
            .mail(ConsoleMail::new())
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


