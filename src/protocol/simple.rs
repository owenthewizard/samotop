use std::io;

// ToDo: Streaming
use model::request::SmtpCommand;
use model::response::*;
use protocol::codec::SmtpCodec;
use protocol::parser::*;
use protocol::socket::NetSocket;
use protocol::writer::SmtpSerializer;
use std::time::SystemTime;
use tokio_io::codec::Framed;
use tokio_proto::pipeline::ServerProto;

pub struct SmtpProto;

impl<T: NetSocket + 'static> ServerProto<T> for SmtpProto {
    /// For this protocol style, `Request` matches the `Item` type of the codec's `Decoder`
    type Request = SmtpCommand;

    /// For this protocol style, `Response` matches the `Item` type of the codec's `Encoder`
    type Response = SmtpReply;

    /// A bit of boilerplate to hook in the codec:
    type Transport = Framed<T, SmtpCodec<'static>>;
    type BindTransport = Result<Self::Transport, io::Error>;
    fn bind_transport(&self, io: T) -> Self::BindTransport {
        // save local and remote socket address so we can use it in the codec
        let local_addr = io.local_addr().ok();
        let peer_addr = io.peer_addr().ok();
        let established = SystemTime::now();
        trace!("@{:?} {:?} -> {:?}", established, peer_addr, local_addr);
        //let since_the_epoch = start
        //    .duration_since(UNIX_EPOCH)
        //    .expect("Time went backwards");
        //println!("{:?}", since_the_epoch);

        Ok(io.framed(SmtpCodec::new(
            SmtpParser::session_parser(),
            SmtpSerializer::answer_serializer(),
            local_addr,
            peer_addr,
            established,
        )))
    }
}
