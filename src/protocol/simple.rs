use std::io;

// ToDo: Streaming
use tokio_proto::pipeline::ServerProto;
use tokio_io::codec::Framed;
use protocol::codec::SmtpCodec;
use protocol::parser::*;
use protocol::writer::SmtpSerializer;
use protocol::socket::NetSocket;
use model::request::SmtpCommand;
use model::response::*;

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
        Ok(io.framed(SmtpCodec::new(
            SmtpParser::session_parser(),
            SmtpSerializer::answer_serializer(),
            local_addr,
            peer_addr,
        )))
    }
}
