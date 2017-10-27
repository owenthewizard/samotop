use std::io;
use tokio_io::codec::Framed;
use tokio_proto::pipeline::ServerProto;
use protocol::codec::SmtpCodec;
use protocol::parser::SmtpParser;
use protocol::writer::SmtpSerializer;
use model::request::{SmtpInput, SmtpConnection};
use model::response::SmtpReply;
use protocol::socket::NetSocket;
use protocol::simple::transport::InitFrameTransport;


pub struct SmtpBaseProto;

impl<TIO> ServerProto<TIO> for SmtpBaseProto
where
    TIO: 'static + NetSocket,
{
    type Request = SmtpInput;
    type Response = SmtpReply;
    type Transport = InitFrameTransport<Framed<TIO, SmtpCodec<'static>>, SmtpInput>;
    type BindTransport = io::Result<Self::Transport>;
    // TODO: Make it into a Future to free the listener loop sooner
    fn bind_transport(&self, io: TIO) -> Self::BindTransport {
        // save local and remote socket address so we can use it as the first frame
        let initframe = SmtpInput::Connect(SmtpConnection {
            local_addr: io.local_addr().ok(),
            peer_addr: io.peer_addr().ok(),
        });
        let codec = SmtpCodec::new(
            SmtpParser::session_parser(),
            SmtpSerializer::answer_serializer(),
        );
        let upstream = io.framed(codec);
        let transport = InitFrameTransport::new(upstream, initframe);
        Ok(transport)
    }
}
